use crate::modbus::{self};
use crate::Error;
use serde::Deserialize;
use tokio::{select, sync::mpsc};
use tokio_modbus::client::{rtu, tcp, Context as ModbusClient};
use tracing::{debug, error};

use crate::{mqtt, shutdown::Shutdown};

pub(crate) async fn run(
    config: Config,
    mqtt: mqtt::Handle,
    shutdown: Shutdown,
) -> crate::Result<Handle> {
    let (handle_tx, handle_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        // Can unwrap because if MQTT handler is bad, we have nothing to do here.
        mqtt.publish("state", "connecting").await.unwrap();

        match config.settings.connect(config.unit).await {
            Ok(client) => {
                // Can unwrap because if MQTT handler is bad, we have nothing to do here.
                mqtt.publish("state", "connected").await.unwrap();

                // Create handle and send to caller
                let (tx, rx) = mpsc::channel(32);
                handle_tx.send(Ok(Handle { tx })).unwrap();

                let conn = Connection {
                    client,
                    mqtt,
                    shutdown,
                    rx,
                };

                if let Err(error) = conn.run().await {
                    error!(?error, "Modbus connection failed");
                }
            }
            Err(error) => handle_tx.send(Err(error.into())).unwrap(),
        }
    });

    handle_rx.await.map_err(|_| crate::Error::RecvError)?
}

struct Connection {
    client: ModbusClient,
    mqtt: mqtt::Handle,
    shutdown: Shutdown,
    rx: mpsc::Receiver<Message>,
}

#[derive(Debug)]
pub struct Handle {
    tx: mpsc::Sender<Message>,
}

#[derive(Debug)]
enum Message {}

impl Connection {
    pub async fn run(mut self) -> crate::Result<()> {
        loop {
            select! {
                Some(msg) = self.rx.recv() => { debug!(?msg); },
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(flatten)]
    pub settings: ModbusProto,

    #[serde(
        alias = "slave",
        default = "tokio_modbus::slave::Slave::broadcast",
        with = "Unit"
    )]
    pub unit: modbus::Unit,

    #[serde(default)]
    pub address_offset: i8,
}

#[derive(Deserialize)]
#[serde(remote = "tokio_modbus::slave::Slave")]
pub(crate) struct Unit(crate::modbus::UnitId);

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "proto", rename_all = "lowercase")]
pub(crate) enum ModbusProto {
    #[cfg(feature = "tcp")]
    Tcp {
        host: String,

        #[serde(default = "default_modbus_port")]
        port: u16,
    },
    #[cfg(feature = "rtu")]
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
    #[cfg(feature = "winet-s")]
    #[serde(rename = "winet-s")]
    SungrowWiNetS { host: String },

    // Predominantly for if the binary is compiled with no default features for some reason.
    #[serde(other)]
    Unknown,
}

impl ModbusProto {
    // Can we use the "slave context" thing in Modbus to pass the unit later?
    pub async fn connect(&self, unit: modbus::Unit) -> crate::Result<ModbusClient> {
        let client = match *self {
            #[cfg(feature = "winet-s")]
            ModbusProto::SungrowWiNetS { ref host } => {
                tokio_modbus_winets::connect_slave(host, unit).await?
            }

            #[cfg(feature = "tcp")]
            ModbusProto::Tcp { ref host, port } => {
                let socket_addr = format!("{}:{}", host, port).parse()?;
                tcp::connect_slave(socket_addr, unit).await?
            }

            #[cfg(feature = "rtu")]
            ModbusProto::Rtu {
                ref tty,
                baud_rate,
                data_bits,
                stop_bits,
                flow_control,
                parity,
            } => {
                let builder = tokio_serial::new(tty, baud_rate)
                    .data_bits(data_bits)
                    .flow_control(flow_control)
                    .parity(parity)
                    .stop_bits(stop_bits);
                let port = tokio_serial::SerialStream::open(&builder)?;
                rtu::connect_slave(port, unit).await?
            }

            ModbusProto::Unknown => {
                error!("Unrecognised protocol");
                Err(Error::UnrecognisedModbusProtocol)?
            }
        };
        Ok(client)
    }
}

pub(crate) fn default_modbus_port() -> u16 {
    502
}

#[cfg(feature = "rtu")]
pub(crate) fn default_modbus_data_bits() -> tokio_serial::DataBits {
    tokio_serial::DataBits::Eight
}

#[cfg(feature = "rtu")]
pub(crate) fn default_modbus_stop_bits() -> tokio_serial::StopBits {
    tokio_serial::StopBits::One
}

#[cfg(feature = "rtu")]
pub(crate) fn default_modbus_flow_control() -> tokio_serial::FlowControl {
    tokio_serial::FlowControl::None
}

#[cfg(feature = "rtu")]
pub(crate) fn default_modbus_parity() -> tokio_serial::Parity {
    tokio_serial::Parity::None
}
