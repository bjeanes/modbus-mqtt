use serde::Serialize;

pub mod config;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectState {
    Connected,
    Disconnected,
    Errored,
}

#[derive(Serialize)]
pub struct ConnectStatus {
    #[serde(flatten)]
    pub connect: config::Connect,
    pub status: ConnectState,
}

pub type UnitId = tokio_modbus::prelude::SlaveId;
pub type Unit = tokio_modbus::prelude::Slave;
