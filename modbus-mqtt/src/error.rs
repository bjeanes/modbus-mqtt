use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    MQTTOptionError(#[from] rumqttc::OptionError),

    #[error(transparent)]
    MQTTClientError(#[from] rumqttc::ClientError),

    #[error(transparent)]
    MQTTConnectionError(#[from] rumqttc::ConnectionError),

    #[error(transparent)]
    InvalidSocketAddr(#[from] std::net::AddrParseError),

    #[error(transparent)]
    SerialError(#[from] tokio_serial::Error),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    JSONError(#[from] serde_json::Error),

    #[error("RecvError")]
    RecvError,

    #[error("SendError")]
    SendError,

    #[error("Unrecognised modbus protocol")]
    UnrecognisedModbusProtocol,

    #[error("{0}")]
    Other(std::borrow::Cow<'static, str>),

    #[error("Unknown")]
    Unknown,
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Other(s.into())
    }
}
impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Self::Other(s.into())
    }
}
