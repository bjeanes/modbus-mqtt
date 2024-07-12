use serde::Deserialize;
use serde_aux::prelude::*;
use std::time::Duration;
use thiserror::Error;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    WebsocketErr(#[from] tungstenite::error::Error),

    #[error(transparent)]
    HttpErr(#[from] reqwest::Error),

    #[error(transparent)]
    HttpHdrErr(#[from] reqwest::header::InvalidHeaderValue),

    // Thank you stranger https://github.com/dtolnay/thiserror/pull/175
    #[error("{code}{}", match .message {
        Some(msg) => format!(" - {}", &msg),
        None => "".to_owned(),
    })]
    SungrowError { code: u16, message: Option<String> },

    #[error(transparent)]
    JSONError(#[from] serde_json::Error),

    #[error("Expected attached data")]
    ExpectedData,

    #[error("No token")]
    NoToken,
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        use std::io::ErrorKind;
        // TODO: Likely there are reasonable mappings from some of our errors to specific io Errors but, for now, this
        // is just so tokio_modbus-winets can fail conveniently.
        std::io::Error::new(ErrorKind::Other, e)
    }
}

#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    host: String,
    username: String,
    password: String,
    token: Option<String>,
    devices: Vec<Device>,
}

pub struct ClientBuilder {
    host: String,
    username: String,
    password: String,
    token: Option<String>,
    user_agent: String,

    read_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    timeout: Option<Duration>,
}

// These are Sungrow's default passwords on the WiNet-S, but they are user-changeable
static DEFAULT_USERNAME: &str = "admin";
static DEFAULT_PASSWORD: &str = "pw8888";
static DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

impl ClientBuilder {
    pub fn new(host: String) -> Self {
        Self {
            host: host.into(),
            username: DEFAULT_USERNAME.into(),
            password: DEFAULT_PASSWORD.into(),
            user_agent: DEFAULT_USER_AGENT.into(),
            token: None,

            connect_timeout: Some(Duration::from_secs(1)),
            read_timeout: Some(Duration::from_secs(1)),
            timeout: Some(Duration::from_secs(1)),
        }
    }

    pub fn build(self) -> Result<Client> {
        use reqwest::header;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(header::ACCEPT, "application/json".parse()?);
        headers.insert(header::CONNECTION, "keep-alive".parse()?);

        let mut http_builder = reqwest::ClientBuilder::new()
            .user_agent(header::HeaderValue::from_str(&self.user_agent)?)
            .default_headers(headers)
            .pool_max_idle_per_host(1)
            .redirect(reqwest::redirect::Policy::none())
            .referer(false);

        if let Some(timeout) = self.timeout {
            http_builder = http_builder.timeout(timeout);
        }
        if let Some(timeout) = self.connect_timeout {
            http_builder = http_builder.connect_timeout(timeout);
        }
        if let Some(timeout) = self.read_timeout {
            http_builder = http_builder.read_timeout(timeout);
        }

        let http = http_builder.build()?;

        Ok(Client {
            host: self.host,
            username: self.username.into(),
            password: self.password.into(),
            token: self.token.into(),
            devices: vec![],
            http,
        })
    }

    pub fn username(mut self, username: String) -> Self {
        self.username = username;
        self
    }

    pub fn password(mut self, password: String) -> Self {
        self.password = password;
        self
    }

    pub fn token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn read_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.read_timeout = timeout;
        self
    }

    pub fn connect_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}

type Result<T> = std::result::Result<T, Error>;

impl Client {
    const WS_PORT: u16 = 8082;

    async fn token(&mut self) -> Result<String> {
        if self.token.is_none() {
            self.token = Some(self.get_token().await?);
        }

        Ok(self.token.as_ref().expect("no token").clone())
    }

    async fn get_token(&self) -> Result<String> {
        use futures_util::SinkExt;
        use futures_util::StreamExt;
        use serde_json::json;

        let ws_url = format!("ws://{}:{}/ws/home/overview", &self.host, Self::WS_PORT);
        debug!(%ws_url, "Connecting to WiNet-S websocket");

        let (mut ws, _) = connect_async(ws_url).await?;

        let connect = Message::Text(
            json!({
                "lang": "en_us",
                "token": self.token.as_deref().unwrap_or_default(),
                "service": "connect"
            })
            .to_string(),
        );
        debug!(%connect, "Sending connect message");
        ws.send(connect).await?;

        while let Some(Ok(Message::Text(msg))) = ws.next().await {
            debug!(%msg, "Got WS message");

            if let SungrowResult {
                code: 1,
                data: Some(ResultData::WebSocketMessage(msg)),
                ..
            } = serde_json::from_str(&msg)?
            {
                match msg {
                    WebSocketMessage::Connect { token: Some(token) } => {
                        let login = Message::Text(
                            json!({
                                "lang": "en_us",
                                "token": token,
                                "service": "login",
                                "username": &self.username,
                                "passwd": &self.password,
                            })
                            .to_string(),
                        );
                        debug!(%login, "Sending login message");
                        ws.send(login).await?;
                    }
                    WebSocketMessage::Login { token } => {
                        return Ok(token.expect(
                            "Login message should have a token, if not an error response",
                        ));
                    }
                    message => {
                        debug!(?message, "Got other message");
                    }
                }
            }
        }

        Err(Error::NoToken)
    }

    pub async fn connect(&mut self) -> Result<()> {
        let data: ResultData = parse_response(
            self.http
                .post(format!("http://{}/inverter/list", &self.host))
                .send()
                .await?,
        )
        .await?;

        if let ResultData::DeviceList(ResultList { items, .. }) = data {
            self.devices = items;
        } else {
            return Err(Error::ExpectedData);
        }

        self.token = Some(self.get_token().await?);
        Ok(())
    }

    // #[tracing::instrument(level = "debug")]
    pub async fn read_register(
        &mut self,
        register_type: RegisterType,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>> {
        let token = self.token().await?;

        // FIXME: find device by phys_addr
        let device = &self.devices[0];

        let request = self
            .http
            .get(format!("http://{}{}", &self.host, "/device/getParam"))
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&[
                ("dev_id", device.dev_id),
                ("dev_type", device.dev_type),
                ("param_type", register_type.param()),
                ("type", 3),
            ])
            .query(&[
                ("dev_code", device.dev_code),
                ("param_addr", address),
                ("param_num", count),
            ])
            .query(&[("lang", "en_us"), ("token", &token)]);
        debug!(?request, "sending request");
        let response = request.send().await?;

        let result = parse_response(response).await?;

        if let ResultData::GetParam { param_value } = result {
            Ok(param_value)
        } else {
            Err(Error::ExpectedData)
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn write_register(&mut self, address: u16, data: &[u16]) -> Result<()> {
        if data.is_empty() {
            return Err(Error::ExpectedData);
        }
        // FIXME: find device by phys_addr
        let device = &self.devices[0];

        use serde_json::json;
        let body = json!({
            "dev_id": device.dev_id,
            "dev_type": device.dev_type,
            "dev_code": device.dev_code,
            "param_addr": address.to_string(),
            "param_size": data.len().to_string(),
            "param_value": data[0].to_string(),
            "lang": "en_us",
            "token": &self.token().await?,
        });
        let request = self
            .http
            .post(format!("http://{}{}", &self.host, "/device/setParam"))
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body);
        let response = request.send().await?;
        parse_response(response).await?;
        Ok(())
    }

    pub async fn running_state(&mut self) -> Result<RunningState> {
        let raw = *self
            .read_register(RegisterType::Input, 13001, 1)
            .await?
            .first()
            .ok_or(Error::ExpectedData)?;
        let bits: RunningStateBits = raw.into();

        let battery_state = if bits.intersects(RunningStateBits::BatteryCharging) {
            BatteryState::Charging
        } else if bits.intersects(RunningStateBits::BatteryDischarging) {
            BatteryState::Discharging
        } else {
            BatteryState::Inactive
        };

        let trading_state = if bits.intersects(RunningStateBits::ImportingPower) {
            TradingState::Importing
        } else if bits.intersects(RunningStateBits::ExportingPower) {
            TradingState::Exporting
        } else {
            TradingState::Inactive
        };

        Ok(RunningState {
            battery_state,
            trading_state,
            generating_pv_power: bits.intersects(RunningStateBits::GeneratingPVPower),
            positive_load_power: bits.intersects(RunningStateBits::LoadActive),
            power_generated_from_load: bits.intersects(RunningStateBits::GeneratingPVPower),
            state: bits,
        })
    }
}

#[derive(Debug)]
pub enum BatteryState {
    Charging,
    Discharging,
    Inactive,
}

#[derive(Debug)]
pub enum TradingState {
    Importing,
    Exporting,
    Inactive,
}

#[derive(Debug)]
pub struct RunningState {
    state: RunningStateBits,
    pub battery_state: BatteryState,
    pub trading_state: TradingState,
    pub generating_pv_power: bool,
    pub positive_load_power: bool,
    pub power_generated_from_load: bool,
}

impl RunningState {
    pub fn raw(&self) -> RunningStateBits {
        self.state
    }
}

// See Appendix 1.2 of Sungrow modbus documentation for hybrid inverters
#[bitmask_enum::bitmask(u16)]
pub enum RunningStateBits {
    GeneratingPVPower = 0b00000001,
    BatteryCharging = 0b00000010,
    BatteryDischarging = 0b00000100,
    LoadActive = 0b00001000,
    LoadReactive = 0b00000000,
    ExportingPower = 0b00010000,
    ImportingPower = 0b00100000,
    PowerGeneratedFromLoad = 0b0100000,
}

#[tracing::instrument(level = "debug")]
async fn parse_response<T>(response: reqwest::Response) -> Result<T>
where
    Result<T>: From<SungrowResult>,
{
    let body = response.text().await?;
    debug!(%body, "parsing");
    let sg_result = serde_json::from_slice::<SungrowResult>(body.as_bytes());
    sg_result?.into()
}

#[derive(Debug)]
pub enum RegisterType {
    Input,
    Holding,
}

impl RegisterType {
    fn param(&self) -> u8 {
        match self {
            Self::Input => 0,
            Self::Holding => 1,
        }
    }
}

// {
// 		"id":	1,
// 		"dev_id":	1,
// 		"dev_code":	3343,
// 		"dev_type":	35,
// 		"dev_procotol":	2,
// 		"inv_type":	0,
// 		"dev_sn":	"REDACTED",
// 		"dev_name":	"SH5.0RS(COM1-001)",
// 		"dev_model":	"SH5.0RS",
// 		"port_name":	"COM1",
// 		"phys_addr":	"1",
// 		"logc_addr":	"1",
// 		"link_status":	1,
// 		"init_status":	1,
// 		"dev_special":	"0",
// 		"list":	[]
// }
#[derive(Clone, Debug, Deserialize)]
struct Device {
    dev_id: u8,
    dev_code: u16,

    // Available from `GET /device/getType`:
    //
    // {
    //     "result_code":  1,
    //     "result_msg":   "success",
    //     "result_data":  {
    //             "count":        5,
    //             "list": [{
    //                             "name": "I18N_COMMON_STRING_INVERTER",
    //                             "value":        1
    //                     }, {
    //                             "name": "I18N_COMMON_SOLAR_INVERTER",
    //                             "value":        21
    //                     }, {
    //                             "name": "I18N_COMMON_STORE_INVERTER",
    //                             "value":        35
    //                     }, {
    //                             "name": "I18N_COMMON_AMMETER",
    //                             "value":        18
    //                     }, {
    //                             "name": "I18N_COMMON_CHARGING_PILE",
    //                             "value":        46
    //                     }]
    //     }
    // }
    //
    // TODO: Extract into enum represented by underlying number?
    dev_type: u8,

    // unit/slave ID
    #[allow(dead_code)]
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
    phys_addr: u8,
    // UNUSED:
    //
    // id: u8,
    // dev_protocol: u8,
    // dev_sn: String,
    // dev_model: String,
    // port_name: String,
    // logc_address: String,
    // link_status: u8,
    // init_status: u8,
    // dev_special: String,
    // list: Option<Vec<()>> // unknown
}

#[test]
fn test_deserialize_device() {
    let json = r#"{
        "id":	1,
        "dev_id":	1,
        "dev_code":	3343,
        "dev_type":	35,
        "dev_procotol":	2,
        "inv_type":	0,
        "dev_sn":	"REDACTED",
        "dev_name":	"SH5.0RS(COM1-001)",
        "dev_model":	"SH5.0RS",
        "port_name":	"COM1",
        "phys_addr":	"1",
        "logc_addr":	"1",
        "link_status":	1,
        "init_status":	1,
        "dev_special":	"0"
    }"#;

    let dev: Device = serde_json::from_str(json).unwrap();

    assert!(matches!(
        dev,
        Device {
            dev_id: 1,
            dev_code: 3343,
            dev_type: 35,
            phys_addr: 1
        }
    ));
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "service", rename_all = "lowercase")]
enum WebSocketMessage {
    Connect {
        token: Option<String>,
        // uid: u8,
        // tips_disable: u8,
        // virgin_flag: u8,
        // isFirstLogin: u8,
        // forceModifyPasswd: u8,
    },
    Login {
        token: Option<String>,
    },

    // DeviceList { list: Vec<Device> },

    // Not yet used:
    // State,  // system state
    // Real,   // real time info
    // Notice, // on some error messages?
    // Statistics,
    // Runtime,
    // Local,
    // Fault,
    // #[serde(rename = "proto_modbus104")]
    // Modbus,
    Other,
}

#[derive(Clone, Debug, Deserialize)]
struct ResultList<T> {
    // count: u16,
    #[serde(rename = "list")]
    items: Vec<T>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ResultData {
    // TODO: custom deserializer into words
    GetParam {
        #[serde(deserialize_with = "words_from_string")]
        param_value: Vec<u16>,
    },
    DeviceList(ResultList<Device>),
    WebSocketMessage(WebSocketMessage),

    // // String = name  - http://<host>/i18n/en_US.properties has the translations for these item names
    // // i32 = value    - unclear if this is always an int, so making this a JSON::Value for now
    // GetType(ResultList<(String, serde_json::Value)>),
    // Product {
    //     #[serde(rename = "product_name")]
    //     name: String,
    //     #[serde(rename = "product_code")]
    //     code: u8,
    // },
    Other,
}

#[test]
fn test_deserialize_get_param() {
    let json = r#"{"param_value":  "82 00 "}"#;
    let data: ResultData = serde_json::from_str(json).unwrap();
    assert!(matches!(data, ResultData::GetParam { .. }));

    let json = r#"{
        "result_code":  1,
        "result_msg":   "success",
        "result_data":  {
                "param_value":  "82 00 "
        }
    }"#;

    let data: SungrowResult = serde_json::from_str(json).unwrap();
    assert!(matches!(
        data,
        SungrowResult {
            code: 1,
            message: Some(m),
            data: Some(ResultData::GetParam { .. })
        } if m == "success"
    ));
}

// TODO: can I make this an _actual_ `Result<ResultData, SungrowError>`?
//         - if code == 1, it is Ok(SungrowData), otherwise create error from code and message?
#[derive(Clone, Debug, Deserialize)]
struct SungrowResult {
    // 1 = success
    // 100 = hit user limit?
    //      {
    //      	"result_code":	100,
    //      	"result_msg":	"normal user limit",
    //      	"result_data":	{
    //      		"service":	"notice"
    //      	}
    //      }
    #[serde(rename = "result_code")]
    code: u16,

    #[serde(rename = "result_msg")]
    // http://<host>/i18n/en_US.properties has the translations for messages (only ones which start with I18N_*)
    message: Option<String>, // at least one result I saw (code = 200 at the time) had no message :\

    #[serde(rename = "result_data")]
    data: Option<ResultData>,
}

impl From<SungrowResult> for Result<Option<ResultData>> {
    fn from(sg_result: SungrowResult) -> Self {
        match sg_result {
            SungrowResult { code: 1, data, .. } => Ok(data),
            SungrowResult { code, message, .. } => Err(Error::SungrowError { code, message }),
        }
    }
}
impl From<SungrowResult> for Result<ResultData> {
    fn from(sg_result: SungrowResult) -> Self {
        let data: Result<Option<ResultData>> = sg_result.into();

        if let Some(data) = data? {
            Ok(data)
        } else {
            Err(Error::ExpectedData)
        }
    }
}
impl From<SungrowResult> for Result<()> {
    fn from(sg_result: SungrowResult) -> Self {
        let data: Result<Option<ResultData>> = sg_result.into();
        data.map(|_| ())
    }
}

// WiNet-S returns data encoded as space-separated hex byte string. E.g.:
//
//      "aa bb cc dd " (yes, including trailing whitespace)
//
// Modbus uses u16 "words" instead of bytes, and the data above should always represent this, so we can take groups
// of 2 and consume them as a hex-represented u16.
fn words_from_string<'de, D>(deserializer: D) -> std::result::Result<Vec<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    StringOrVecToVec::new(' ', |s| u8::from_str_radix(s, 16), true).into_deserializer()(
        deserializer,
    )
    .map(|vec| {
        vec.chunks_exact(2)
            .map(|bytes| u16::from_be_bytes(bytes.try_into().unwrap()))
            .collect()
    })
}

#[test]
fn test_words_from_string() {
    #[derive(serde::Deserialize, Debug)]
    struct MyStruct {
        #[serde(deserialize_with = "words_from_string")]
        list: Vec<u16>,
    }

    let s = r#" { "list": "00 AA 00 01 00 0D 00 1E 00 0F 00 00 00 55 " } "#;
    let a: MyStruct = serde_json::from_str(s).unwrap();
    assert_eq!(
        &a.list,
        &[0x00AA, 0x0001, 0x000D, 0x001E, 0x000F, 0x0000, 0x0055]
    );
}
