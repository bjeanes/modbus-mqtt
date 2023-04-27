use serde::Deserialize;
use serde_aux::prelude::*;
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
    token: String,
    devices: Vec<Device>,
}

const WS_PORT: u16 = 8082;

type Result<T> = std::result::Result<T, Error>;

impl Client {
    pub async fn new<H>(host: H) -> Result<Self>
    where
        H: Into<String>,
    {
        let host = host.into();
        let ws_url = format!("ws://{}:{}/ws/home/overview", &host, WS_PORT);

        use futures_util::SinkExt;
        use futures_util::StreamExt;
        let (mut ws, _) = connect_async(ws_url).await?;

        ws.send(Message::Text(
            serde_json::json!({"lang":"en_us","token":"","service":"connect"}).to_string(),
        ))
        .await?;

        // TODO: maintan WS connection, pinging and watching for updated tokens
        let token = if_chain::if_chain! {
            if let Some(Ok(Message::Text(msg))) = ws.next().await ;
            if let Ok(value) = serde_json::from_str::<SungrowResult>(&msg);
            if let Some(ResultData::WebSocketMessage(WebSocketMessage::Connect { token })) = value.data;
            then {
                debug!(token, "Got WiNet-S token");
                token
            } else {
                // TODO: it might be that we get some other WS messages here that are fine so we might need to take a
                // few WS messages to find the token.
                return Err(Error::NoToken);
            }
        };
        Self::new_with_token(host, token).await
    }

    pub async fn new_with_token<H>(host: H, token: String) -> Result<Self>
    where
        H: Into<String>,
    {
        let host = host.into();
        let http = reqwest::Client::new();

        let data: ResultData = parse_response(
            http.post(format!("http://{}/inverter/list", &host))
                .send()
                .await?,
        )
        .await?;

        if let ResultData::DeviceList(ResultList { items, .. }) = data {
            Ok(Client {
                token,
                devices: items,
                host,
                http,
            })
        } else {
            Err(Error::ExpectedData)
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn read_register(
        &self,
        register_type: RegisterType,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>> {
        // FIXME: find device by phys_addr
        let device = &self.devices[0];

        #[derive(serde::Serialize)]
        struct Params {
            #[serde(rename = "type")]
            type_: u8,
            dev_id: u8,
            dev_type: u8,
            dev_code: u16,
            param_type: u8,
            param_addr: u16,
            param_num: u16,
        }
        let request = self.get("/device/getParam").query(&Params {
            type_: 3,
            dev_id: device.dev_id,
            dev_type: device.dev_type,
            dev_code: device.dev_code,
            param_type: register_type.param(),
            param_addr: address,
            param_num: count,
        });
        let response = request.send().await?;

        let result = parse_response(response).await?;

        if let ResultData::GetParam { param_value } = result {
            Ok(param_value)
        } else {
            Err(Error::ExpectedData)
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn write_register(&self, address: u16, data: &[u16]) -> Result<()> {
        if data.is_empty() {
            return Err(Error::ExpectedData);
        }
        // FIXME: find device by phys_addr
        let device = &self.devices[0];

        use serde_json::json;
        let body = json!({
            "lang": "en_us",
            "token": &self.token,
            "dev_id": device.dev_id,
            "dev_type": device.dev_type,
            "dev_code": device.dev_code,
            "param_addr": address.to_string(),
            "param_size": data.len().to_string(),
            "param_value": data[0].to_string(),
        });
        let request = self
            .http
            .post(format!("http://{}{}", &self.host, "/device/setParam"))
            .json(&body);
        let response = request.send().await?;
        parse_response(response).await?;
        Ok(())
    }

    pub async fn running_state(&self) -> Result<RunningState> {
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

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("http://{}{}", &self.host, path))
            .query(&[("lang", "en_us"), ("token", self.token.as_str())])
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
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
#[serde(tag = "service", rename_all = "lowercase")]
enum WebSocketMessage {
    Connect { token: String },

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

#[derive(Debug, Deserialize)]
struct ResultList<T> {
    // count: u16,
    #[serde(rename = "list")]
    items: Vec<T>,
}

#[derive(Debug, Deserialize)]
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
#[derive(Deserialize)]
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
