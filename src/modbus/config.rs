use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
use serde_json::json;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModbusProto {
    Tcp {
        host: String,

        #[serde(default = "default_modbus_port")]
        port: u16,
    },
    #[serde(rename_all = "lowercase")]
    Rtu {
        // tty: std::path::PathBuf,
        tty: String,
        baud_rate: u32,

        #[serde(default = "default_modbus_data_bits")]
        data_bits: tokio_serial::DataBits, // TODO: allow this to be represented as a number instead of string

        #[serde(default = "default_modbus_stop_bits")]
        stop_bits: tokio_serial::StopBits, // TODO: allow this to be represented as a number instead of string

        #[serde(default = "default_modbus_flow_control")]
        flow_control: tokio_serial::FlowControl,

        #[serde(default = "default_modbus_parity")]
        parity: tokio_serial::Parity,
    },
}

fn default_modbus_port() -> u16 {
    502
}

fn default_modbus_data_bits() -> tokio_serial::DataBits {
    tokio_serial::DataBits::Eight
}

fn default_modbus_stop_bits() -> tokio_serial::StopBits {
    tokio_serial::StopBits::One
}

fn default_modbus_flow_control() -> tokio_serial::FlowControl {
    tokio_serial::FlowControl::None
}

fn default_modbus_parity() -> tokio_serial::Parity {
    tokio_serial::Parity::None
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", default)]
pub struct RegisterNumericAdjustment {
    pub scale: i8, // powers of 10 (0 = no adjustment, 1 = x10, -1 = /10)
    pub offset: i8,
    // precision: Option<u8>,
}

impl Default for RegisterNumericAdjustment {
    fn default() -> Self {
        Self {
            scale: 0,
            offset: 0,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterNumeric {
    U8,
    #[default]
    U16,
    U32,
    U64,

    #[serde(alias = "s8")]
    I8,
    #[serde(alias = "s16")]
    I16,
    #[serde(alias = "s32")]
    I32,
    #[serde(alias = "s64")]
    I64,

    F32,
    F64,
}

impl RegisterNumeric {
    // Modbus limits sequential reads to 125 apparently, so 8-bit should be fine - https://github.com/slowtec/tokio-modbus/issues/112#issuecomment-1095316069=
    fn size(&self) -> u8 {
        use RegisterNumeric::*;
        // Each Modbus register holds 16-bits, so count is half what the byte count would be
        match self {
            U8 | I8 => 1,
            U16 | I16 => 1,
            U32 | I32 | F32 => 2,
            U64 | I64 | F64 => 4,
        }
    }

    fn type_name(&self) -> String {
        format!("{:?}", *self).to_lowercase()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "string")]
pub struct RegisterString {
    length: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "array")]
pub struct RegisterArray {
    count: u8,

    #[serde(default)]
    of: RegisterNumeric,

    // Arrays are only of numeric types, so we can apply an adjustment here
    #[serde(flatten, skip_serializing_if = "IsDefault::is_default")]
    adjust: RegisterNumericAdjustment,
}

impl Default for RegisterArray {
    fn default() -> Self {
        Self {
            count: 1,
            of: Default::default(),
            adjust: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegisterValueType {
    Numeric {
        #[serde(rename = "type", default)]
        of: RegisterNumeric,

        #[serde(flatten, skip_serializing_if = "IsDefault::is_default")]
        adjust: RegisterNumericAdjustment,
    },
    Array(RegisterArray),
    String(RegisterString),
}

impl RegisterValueType {
    pub fn type_name(&self) -> String {
        match *self {
            RegisterValueType::Numeric { ref of, .. } => of.type_name(),
            RegisterValueType::Array(_) => "array".to_owned(),
            RegisterValueType::String(_) => "string".to_owned(),
        }
    }
}

impl Default for RegisterValueType {
    fn default() -> Self {
        RegisterValueType::Numeric {
            of: Default::default(),
            adjust: Default::default(),
        }
    }
}

impl RegisterValueType {
    // Modbus limits sequential reads to 125 apparently, so 8-bit should be fine - https://github.com/slowtec/tokio-modbus/issues/112#issuecomment-1095316069=
    pub fn size(&self) -> u8 {
        use RegisterValueType::*;

        match self {
            Numeric { of, .. } => of.size(),
            String(RegisterString { length }) => *length,
            Array(RegisterArray { of, count, .. }) => of.size() * count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Swap(pub bool);

impl Default for Swap {
    fn default() -> Self {
        Self(false)
    }
}

trait IsDefault {
    fn is_default(&self) -> bool;
}
impl<T> IsDefault for T
where
    T: Default + PartialEq,
{
    fn is_default(&self) -> bool {
        *self == Default::default()
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct RegisterParse {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub swap_bytes: Swap,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub swap_words: Swap,

    #[serde(flatten, skip_serializing_if = "IsDefault::is_default")]
    pub value_type: RegisterValueType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Register {
    pub address: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(flatten, default, skip_serializing_if = "IsDefault::is_default")]
    pub parse: RegisterParse,

    #[serde(
        with = "humantime_serde",
        default = "default_register_interval",
        alias = "period",
        alias = "duration"
    )]
    pub interval: Duration,
}

fn default_register_interval() -> Duration {
    Duration::from_secs(60)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Connect {
    #[serde(flatten)]
    pub settings: ModbusProto,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<Register>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hold: Vec<Register>,

    #[serde(
        alias = "slave",
        default = "tokio_modbus::slave::Slave::broadcast",
        with = "Unit"
    )]
    pub unit: crate::modbus::Unit,

    #[serde(default = "default_address_offset")]
    pub address_offset: i8,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "tokio_modbus::slave::Slave")]
struct Unit(crate::modbus::UnitId);

fn default_address_offset() -> i8 {
    0
}

#[test]
fn parse_minimal_tcp_connect_config() {
    let result = serde_json::from_value::<Connect>(json!({
        "host": "1.1.1.1"
    }));

    let connect = result.unwrap();
    assert!(matches!(
        connect.settings,
        ModbusProto::Tcp {
            ref host,
            port: 502
        } if host == "1.1.1.1"
    ))
}

#[test]
fn parse_full_tcp_connect_config() {
    let _ = serde_json::from_value::<Connect>(json!({
        "host": "10.10.10.219",
        "unit": 1,
        "address_offset": -1,
        "input": [
            {
                "address": 5017,
                "type": "u32",
                "name": "dc_power",
                "swap_words": false,
                "period": "3s"
            },
            {
                "address": 5008,
                "type": "s16",
                "name": "internal_temperature",
                "period": "1m"
            },
            {
                "address": 13008,
                "type": "s32",
                "name": "load_power",
                "swap_words": false,
                "period": "3s"
            },
            {
                "address": 13010,
                "type": "s32",
                "name": "export_power",
                "swap_words": false,
                "period": "3s"
            },
            {
                "address": 13022,
                "name": "battery_power",
                "period": "3s"
            },
            {
                "address": 13023,
                "name": "battery_level",
                "period": "1m"
            },
            {
                "address": 13024,
                "name": "battery_health",
                "period": "10m"
            }
        ],
        "hold": [
            {
                "address": 13058,
                "name": "max_soc",
                "period": "90s"
            },
            {
                "address": 13059,
                "name": "min_soc",
                "period": "90s"
            }
        ]
    }))
    .unwrap();
}

#[test]
fn parse_minimal_rtu_connect_config() {
    let result = serde_json::from_value::<Connect>(json!({
        "tty": "/dev/ttyUSB0",
        "baud_rate": 9600,
    }));

    let connect = result.unwrap();
    use tokio_serial::*;
    assert!(matches!(
        connect.settings,
        ModbusProto::Rtu {
            ref tty,
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            parity: Parity::None,
            ..
        } if tty == "/dev/ttyUSB0"
    ))
}

#[test]
fn parse_complete_rtu_connect_config() {
    let result = serde_json::from_value::<Connect>(json!({
        "tty": "/dev/ttyUSB0",
        "baud_rate": 12800,

        // TODO: make lowercase words work
        "data_bits": "Seven", // TODO: make 7 work
        "stop_bits": "Two", // TODO: make 2 work
        "flow_control": "Software",
        "parity": "Even",
    }));

    let connect = result.unwrap();
    use tokio_serial::*;
    assert!(matches!(
        connect.settings,
        ModbusProto::Rtu {
            ref tty,
            baud_rate: 12800,
            data_bits: DataBits::Seven,
            stop_bits: StopBits::Two,
            flow_control: FlowControl::Software,
            parity: Parity::Even,
            ..
        } if tty == "/dev/ttyUSB0"
    ),);
}

#[test]
fn parse_empty_register_parser_defaults() {
    let empty = serde_json::from_value::<RegisterParse>(json!({}));
    assert!(matches!(
        empty.unwrap(),
        RegisterParse {
            swap_bytes: Swap(false),
            swap_words: Swap(false),
            value_type: RegisterValueType::Numeric {
                of: RegisterNumeric::U16,
                adjust: RegisterNumericAdjustment {
                    scale: 0,
                    offset: 0,
                }
            }
        }
    ));
}

#[test]
fn parse_register_parser_type() {
    let result = serde_json::from_value::<RegisterParse>(json!({
        "type": "s32"
    }));
    assert!(matches!(
        result.unwrap().value_type,
        RegisterValueType::Numeric {
            of: RegisterNumeric::I32,
            ..
        }
    ));
}

#[test]
fn parse_register_parser_array() {
    let result = serde_json::from_value::<RegisterParse>(json!({
        "type": "array",
        "of": "s32",
        "count": 10,
    }));
    let payload = result.unwrap();
    // println!("{:?}", payload);
    // println!("{}", serde_json::to_string_pretty(&payload).unwrap());

    assert!(matches!(
        payload.value_type,
        RegisterValueType::Array(RegisterArray {
            of: RegisterNumeric::I32,
            count: 10,
            ..
        })
    ));
}

#[test]
fn parse_register_parser_array_implicit_u16() {
    let result = serde_json::from_value::<RegisterParse>(json!({
        "type": "array",
        "count": 10,
    }));
    let payload = result.unwrap();
    // println!("{:?}", payload);
    // println!("{}", serde_json::to_string_pretty(&payload).unwrap());

    assert!(matches!(
        payload.value_type,
        RegisterValueType::Array(RegisterArray {
            of: RegisterNumeric::U16,
            count: 10,
            ..
        })
    ));
}

#[test]
fn parse_register_parser_string() {
    let result = serde_json::from_value::<RegisterParse>(json!({
        "type": "string",
        "length": 10,
    }));
    let payload = result.unwrap();
    // println!("{:?}", payload);
    // println!("{}", serde_json::to_string_pretty(&payload).unwrap());

    assert!(matches!(
        payload.value_type,
        RegisterValueType::String(RegisterString { length: 10, .. })
    ));
}

#[test]
fn parse_register_parser_scale_etc() {
    let result = serde_json::from_value::<RegisterParse>(json!({
        "type": "s32",
        "scale": -1,
        "offset": 20,
    }));
    assert!(matches!(
        result.unwrap().value_type,
        RegisterValueType::Numeric {
            of: RegisterNumeric::I32,
            adjust: RegisterNumericAdjustment {
                scale: -1,
                offset: 20
            }
        }
    ));
}
