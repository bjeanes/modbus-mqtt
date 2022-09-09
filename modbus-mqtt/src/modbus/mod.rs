pub mod connection;
pub mod connector;
pub mod register;

pub use connection::Handle;

type Word = u16;

pub type UnitId = tokio_modbus::prelude::SlaveId;
pub type Unit = tokio_modbus::prelude::Slave;
