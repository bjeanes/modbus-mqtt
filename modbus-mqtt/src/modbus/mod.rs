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

pub type UnitId = tokio_modbus::prelude::SlaveId;
pub type Unit = tokio_modbus::prelude::Slave;
