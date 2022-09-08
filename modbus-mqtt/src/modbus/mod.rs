use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::Serialize;

use self::register::{Register, RegisterValueType};

pub mod connection;
pub mod connector;
pub mod register;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectState {
    Connected,
    Disconnected,
    Errored,
}

// #[derive(Serialize)]
// pub struct ConnectStatus {
//     #[serde(flatten)]
//     pub connect: config::Connect,
//     pub status: ConnectState,
// }

pub type UnitId = tokio_modbus::prelude::SlaveId;
pub type Unit = tokio_modbus::prelude::Slave;

impl RegisterValueType {
    pub fn parse_words(&self, words: &[u16]) -> serde_json::Value {
        use self::register::RegisterValueType as T;
        use self::register::{RegisterArray, RegisterNumeric as N, RegisterString};
        use serde_json::json;

        let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_ne_bytes()).collect();

        match *self {
            T::Numeric { ref of, ref adjust } => {
                use rust_decimal::MathematicalOps;
                let scale: Decimal = Decimal::TEN.powi(adjust.scale.into()).normalize();
                let offset = Decimal::from(adjust.offset);
                match of {
                    N::U8 => json!(scale * Decimal::from(bytes[1]) + offset), // or is it 0?
                    N::U16 => json!(scale * Decimal::from(words[0]) + offset),
                    N::U32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(u32::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::U64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(u64::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I8 => {
                        json!(vec![bytes[1]]
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i8::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I16 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i16::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i32::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::I64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale * Decimal::from(i64::from_le_bytes(bytes)) + offset)
                            .ok())
                    }
                    N::F32 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale
                                * Decimal::from_f32(f32::from_le_bytes(bytes)).unwrap()
                                + offset)
                            .ok())
                    }
                    N::F64 => {
                        json!(bytes
                            .try_into()
                            .map(|bytes| scale
                                * Decimal::from_f64(f64::from_le_bytes(bytes)).unwrap()
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
    pub fn parse_words(&self, words: &[u16]) -> serde_json::Value {
        self.parse.value_type.parse_words(words)
    }

    pub fn apply_swaps(&self, words: &[u16]) -> Vec<u16> {
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
#[test]
fn test_parse_1() {
    use self::register::{RegisterParse, Swap};
    use serde_json::json;

    let reg = Register {
        address: 42,
        name: None,
        interval: Default::default(),
        parse: RegisterParse {
            swap_bytes: Swap(false),
            swap_words: Swap(false),
            value_type: RegisterValueType::Numeric {
                of: register::RegisterNumeric::I32,
                adjust: register::RegisterNumericAdjustment {
                    scale: 0,
                    offset: 0,
                },
            },
        },
    };

    assert_eq!(reg.parse_words(&[843, 0]), json!(843));
}
