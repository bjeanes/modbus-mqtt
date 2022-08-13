use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
// TODO: `scale`, `offset`, `precision`
pub enum RegisterFixedValueType {
    U8,
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

impl RegisterFixedValueType {
    // Modbus limits sequential reads to 125 apparently, so 8-bit should be fine - https://github.com/slowtec/tokio-modbus/issues/112#issuecomment-1095316069=
    fn size(&self) -> u8 {
        use RegisterFixedValueType::*;
        // Each Modbus register holds 16-bits, so count is half what the byte count would be
        match self {
            U8 => 1,
            U16 => 1,
            U32 => 2,
            U64 => 4,
            I8 => 1,
            I16 => 1,
            I32 => 2,
            I64 => 4,
            F32 => 2,
            F64 => 4,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterVariableValueType {
    String,
    Array(RegisterFixedValueType),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum RegisterValueType {
    Fixed(RegisterFixedValueType),
    Variable(RegisterVariableValueType, u8),
}

impl RegisterValueType {
    // Modbus limits sequential reads to 125 apparently, so 8-bit should be fine - https://github.com/slowtec/tokio-modbus/issues/112#issuecomment-1095316069=
    pub fn size(&self) -> u8 {
        use RegisterValueType::*;
        use RegisterVariableValueType::*;

        match self {
            Fixed(fixed) => fixed.size(),
            Variable(variable, count) => match variable {
                String => *count,
                Array(fixed) => *count * fixed.size(),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RegisterParse {
    #[serde(default = "default_swap")]
    pub swap_bytes: bool,

    #[serde(default = "default_swap")]
    pub swap_words: bool,

    #[serde(rename = "type", default = "default_value_type")]
    pub value_type: RegisterValueType,
}

fn default_swap() -> bool {
    false
}

fn default_value_type() -> RegisterValueType {
    RegisterValueType::Fixed(RegisterFixedValueType::U16)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Register {
    pub address: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(flatten, default = "default_register_parse")]
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

fn default_register_parse() -> RegisterParse {
    RegisterParse {
        swap_bytes: default_swap(),
        swap_words: default_swap(),
        value_type: default_value_type(),
    }
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
    assert!(result.is_ok());

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
    let result = serde_json::from_value::<Connect>(json!({
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
    }));

    assert!(result.is_ok());
}

#[test]
fn parse_minimal_rtu_connect_config() {
    let result = serde_json::from_value::<Connect>(json!({
        "tty": "/dev/ttyUSB0",
        "baud_rate": 9600,
    }));
    assert!(result.is_ok());

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
