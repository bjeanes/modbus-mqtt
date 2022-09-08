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

#[test]
fn parse_minimal_tcp_connect_config() {
    use serde_json::json;
    let result = serde_json::from_value::<Config>(json!({
        "proto": "tcp",
        "host": "1.1.1.1"
    }));

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
    use serde_json::json;
    let _ = serde_json::from_value::<Config>(json!({
        "proto": "tcp",
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
    }))
    .unwrap();
}

#[test]
fn parse_minimal_rtu_connect_config() {
    use serde_json::json;
    let result = serde_json::from_value::<Config>(json!({
        "proto": "rtu",
        "tty": "/dev/ttyUSB0",
        "baud_rate": 9600,
    }));

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

#[test]
fn parse_complete_rtu_connect_config() {
    use serde_json::json;
    let result = serde_json::from_value::<Config>(json!({
        "proto": "rtu",
        "tty": "/dev/ttyUSB0",
        "baud_rate": 12800,

        // TODO: make lowercase words work
        "data_bits": "Seven", // TODO: make 7 work
        "stop_bits": "Two", // TODO: make 2 work
        "flow_control": "Software",
        "parity": "Even",
    }));

    let connect = result.unwrap();
    use tokio_serial::*;
    assert!(matches!(
        connect.settings,
        ModbusProto::Rtu {
            ref tty,
            baud_rate: 12800,
            data_bits: DataBits::Seven,
            stop_bits: StopBits::Two,
            flow_control: FlowControl::Software,
            parity: Parity::Even,
            ..
        } if tty == "/dev/ttyUSB0"
    ),);
}
