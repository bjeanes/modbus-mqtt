mod shutdown;

pub mod modbus;
pub mod mqtt;
pub mod server;

mod error;
pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
