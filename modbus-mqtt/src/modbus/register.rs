use super::Word;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{
    select,
    sync::mpsc,
    time::{interval, MissedTickBehavior},
};
use tracing::{debug, warn};

pub struct Monitor {
    mqtt: mqtt::Handle,
    modbus: super::Handle,
    address: u16,
    register: Register,
    register_type: RegisterType,
}

impl Monitor {
    pub fn new(
        register: Register,
        register_type: RegisterType,
        address: u16,
        mqtt: mqtt::Handle,
        modbus: super::Handle,
    ) -> Monitor {
        Monitor {
            mqtt,
            register_type,
            register,
            address,
            modbus,
        }
    }

    pub async fn run(self) {
        tokio::spawn(async move {
            let mut interval = interval(self.register.interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;
                if let Ok(words) = self.read().await {
                    debug!(address=%self.address, "type"=?self.register_type, ?words);

                    #[cfg(debug_assertions)]
                    self.mqtt
                        .publish("raw", serde_json::to_vec(&words).unwrap())
                        .await
                        .unwrap();

                    let value = self.register.parse_words(&words);

                    self.mqtt
                        .publish("state", serde_json::to_vec(&value).unwrap())
                        .await
                        .unwrap();
                }
            }
        });
    }

    async fn read(&self) -> crate::Result<Vec<Word>> {
        match self.register_type {
            RegisterType::Input => {
                self.modbus
                    .read_input_register(self.address, self.register.size())
                    .await
            }
            RegisterType::Holding => {
                self.modbus
                    .read_holding_register(self.address, self.register.size())
                    .await
            }
        }
    }
}

pub(crate) async fn subscribe(
    mqtt: &mqtt::Handle,
) -> crate::Result<mpsc::Receiver<(RegisterType, AddressedRegister)>> {
    let (tx, rx) = mpsc::channel(8);
    let mut input_registers = mqtt.subscribe("input/+").await?;
    let mut holding_registers = mqtt.subscribe("holding/+").await?;

    tokio::spawn(async move {
        fn to_register(payload: &Payload) -> crate::Result<AddressedRegister> {
            let Payload { bytes, topic } = payload;
            let address = topic
                .rsplit('/')
                .next()
                .expect("subscribed topic guarantees we have a last segment")
                .parse()?;
            Ok(AddressedRegister {
                address,
                register: serde_json::from_slice(bytes)?,
            })
        }

        loop {
            select! {
                Some(ref payload) = input_registers.recv() => {
                    match to_register(payload) {
                        Ok(register) => if (tx.send((RegisterType::Input, register)).await).is_err() { break; },
                        Err(error) => warn!(?error, def=?payload.bytes, "ignoring invalid input register definition"),
                    }
                },
                Some(ref payload) = holding_registers.recv() => {
                    match to_register(payload) {
                        Ok(register) => if (tx.send((RegisterType::Holding, register)).await).is_err() { break; },
                        Err(error) => warn!(?error, def=?payload.bytes, "ignoring invalid holding register definition"),
                    }
                }
            }
        }
    });

    Ok(rx)
}

#[derive(Clone, Copy, Debug)]
pub enum RegisterType {
    Input,
    Holding,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", default)]
pub struct RegisterNumericAdjustment {
    pub scale: i8, // powers of 10 (0 = no adjustment, 1 = x10, -1 = /10)
    pub offset: i8,
    // precision: Option<u8>,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "string")]
pub struct RegisterString {
    length: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Swap(pub bool);

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

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RegisterParse {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub swap_bytes: Swap,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub swap_words: Swap,

    #[serde(flatten, skip_serializing_if = "IsDefault::is_default")]
    pub value_type: RegisterValueType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Register {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressedRegister {
    pub address: u16,

    #[serde(flatten)]
    pub register: Register,
}

fn default_register_interval() -> Duration {
    Duration::from_secs(60)
}

#[test]
fn parse_empty_register_parser_defaults() {
    use serde_json::json;
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
    use serde_json::json;
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
    use serde_json::json;
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
    use serde_json::json;
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
    use serde_json::json;
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
    use serde_json::json;
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

impl RegisterValueType {
    pub fn parse_words(&self, words: &[u16]) -> serde_json::Value {
        use self::RegisterNumeric as N;
        use rust_decimal::{prelude::FromPrimitive, Decimal, MathematicalOps};
        use serde_json::json;
        use RegisterValueType as T;

        let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_be_bytes()).collect();

        match *self {
            T::Numeric { ref of, ref adjust } => {
                let scale: Decimal = Decimal::TEN.powi(adjust.scale.into()).normalize();
                let offset = Decimal::from(adjust.offset);
                match of {
                    N::U8 => json!(scale * Decimal::from(bytes[1]) + offset), // or is it 0?
                    N::U16 => json!(scale * Decimal::from(words[0]) + offset),
                    N::U32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(u32::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::U64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(u64::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I8 => {
                        json!(vec![bytes[1]]
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i8::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I16 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i16::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i32::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i64::from_be_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::F32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale
                                * Decimal::from_f32(f32::from_be_bytes(bytes)).unwrap()
                                + offset)
                            .ok())
                    }
                    N::F64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale
                                * Decimal::from_f64(f64::from_be_bytes(bytes)).unwrap()
                                + offset)
                            .ok())
                    }
                }
            }
            T::String(RegisterString { .. }) => {
                json!(String::from_utf16_lossy(words))
            }
            T::Array(RegisterArray { .. }) => todo!(),
        }
    }
}

impl Register {
    pub fn size(&self) -> u8 {
        self.parse.value_type.size()
    }

    pub fn parse_words(&self, words: &[u16]) -> serde_json::Value {
        self.parse.value_type.parse_words(&self.apply_swaps(words))
    }

    fn apply_swaps(&self, words: &[u16]) -> Vec<u16> {
        let words: Vec<u16> = if self.parse.swap_bytes.0 {
            words.iter().map(|v| v.swap_bytes()).collect()
        } else {
            words.into()
        };

        if self.parse.swap_words.0 {
            words
                .chunks_exact(2)
                .flat_map(|chunk| vec![chunk[1], chunk[0]])
                .collect()
        } else {
            words
        }
    }
}
#[cfg(test)]
use pretty_assertions::assert_eq;

use crate::mqtt::{self, Payload};
#[test]
fn test_parse_1() {
    use serde_json::json;

    let reg = Register {
        name: None,
        interval: Default::default(),
        parse: RegisterParse {
            swap_bytes: Swap(false),
            swap_words: Swap(true),
            value_type: RegisterValueType::Numeric {
                of: RegisterNumeric::U32,
                adjust: RegisterNumericAdjustment {
                    scale: 0,
                    offset: 0,
                },
            },
        },
    };

    assert_eq!(reg.parse_words(&[843, 0]), json!(843));
}
