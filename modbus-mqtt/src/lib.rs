use rumqttc::{self};

use tracing::error;

use thiserror::Error;

mod shutdown;

pub mod homeassistant;
pub mod modbus;
pub mod mqtt;
pub mod server;

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

pub type Result<T> = std::result::Result<T, Error>;

//             tokio::spawn(async move {
//                 while let Some(command) = modbus_rx.recv().await {
//                 }
//             });

//             use itertools::Itertools;
//             for (duration, registers) in &connect.input.into_iter().group_by(|r| r.interval) {
//                 let registers_prefix = format!("{}/input/{}", topic_prefix, id);

//                 tokio::spawn(watch_registers(
//                     ModbusReadType::Input,
//                     connect.address_offset,
//                     duration,
//                     registers.collect(),
//                     modbus_tx.clone(),
//                     dispatcher.clone(),
//                     registers_prefix,
//                 ));
//             }
//             for (duration, registers) in &connect.hold.into_iter().group_by(|r| r.interval) {
//                 let registers_prefix = format!("{}/hold/{}", topic_prefix, id);

//                 tokio::spawn(watch_registers(
//                     ModbusReadType::Hold,
//                     connect.address_offset,
//                     duration,
//                     registers.collect(),
//                     modbus_tx.clone(),
//                     dispatcher.clone(),
//                     registers_prefix,
//                 ));
//             }
//         }
//         Err(err) => {
//             dispatcher
//                 .send(DispatchCommand::Publish {
//                     topic: format!("{}/status/{}", topic_prefix, id),
//                     payload: serde_json::to_vec(&json!({
//                         "status": ConnectState::Errored,
//                         "error": format!("Invalid config: {}", err),
//                     }))
//                     .unwrap(),
//                 })
//                 .await
//                 .unwrap();
//         }
//     }
// }

// #[tracing::instrument(level = "debug")]
// async fn watch_registers(
//     read_type: ModbusReadType,
//     address_offset: i8,
//     duration: Duration,
//     registers: Vec<modbus::config::Register>,
//     modbus: mpsc::Sender<ModbusCommand>,
//     dispatcher: mpsc::Sender<DispatchCommand>,
//     registers_prefix: String,
// ) -> ! {
//     let mut interval = tokio::time::interval(duration);
//     interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

//     loop {
//         interval.tick().await;
//         for r in registers.iter() {
//             let address = if address_offset >= 0 {
//                 r.address.checked_add(address_offset as u16)
//             } else {
//                 r.address.checked_sub(address_offset.unsigned_abs() as u16)
//             };
//             if let Some(address) = address {
//                 let size = r.parse.value_type.size();
//                 debug!(
//                     name = r.name.as_ref().unwrap_or(&"".to_string()),
//                     address,
//                     size,
//                     register_type = ?read_type,
//                     value_type = r.parse.value_type.type_name(),
//                     "Polling register",
//                 );

//                 let (tx, rx) = oneshot::channel();

//                 modbus
//                     .send(ModbusCommand::Read(read_type, address, size, tx))
//                     .await
//                     .unwrap();

//                 // FIXME: definitely getting errors here that need to be handled
//                 //
//                 //
//                 // Splitting out the two awaits so I can see if all of the above panics come from the same await or some from one vs the other:
//                 let response = rx.await.unwrap(); // await may have errorer on receiving
//                 let words = response.unwrap(); // received message is also a result which may be a (presumably Modbus?) error

//                 let swapped_words = r.apply_swaps(&words);

//                 let value = r.parse_words(&swapped_words);

//                 debug!(
//                     name = r.name.as_ref().unwrap_or(&"".to_string()),
//                     address,
//                     %value,
//                     raw = ?words,
//                     "Received value",
//                 );

//                 let payload = serde_json::to_vec(&json!({ "value": value, "raw": words })).unwrap();

//                 dispatcher
//                     .send(DispatchCommand::Publish {
//                         topic: format!("{}/{}", registers_prefix, r.address),
//                         payload: payload.clone(),
//                     })
//                     .await
//                     .unwrap();

//                 if let Some(name) = &r.name {
//                     dispatcher
//                         .send(DispatchCommand::Publish {
//                             topic: format!("{}/{}", registers_prefix, name),
//                             payload,
//                         })
//                         .await
//                         .unwrap();
//                 }
//             }
//         }
//     }
// }
