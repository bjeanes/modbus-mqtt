use crate::modbus::{self};
use crate::Error;
use rust_decimal::prelude::Zero;
use serde::Deserialize;
use tokio::sync::oneshot;
use tokio::{select, sync::mpsc};
use tokio_modbus::client::{rtu, tcp, Context as ModbusClient};
use tracing::{debug, error, warn};

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

        let address_offset = config.address_offset;

        match config.settings.connect(config.unit).await {
            Ok(client) => {
                // Can unwrap because if MQTT handler is bad, we have nothing to do here.
                mqtt.publish("state", "connected").await.unwrap();

                let (tx, rx) = mpsc::channel(32);

                let conn = Connection {
                    address_offset,
                    client,
                    mqtt: mqtt.clone(),
                    shutdown,
                    rx,
                    tx,
                };

                handle_tx.send(Ok(conn.handle())).unwrap();

                if let Err(error) = conn.run().await {
                    error!(?error, "Modbus connection failed");
                }

                // we are shutting down here, so don't care if this fails
                let _ = mqtt.publish("state", "disconnected").await;
            }
            Err(error) => handle_tx.send(Err(error.into())).unwrap(),
        }
    });

    handle_rx.await.map_err(|_| crate::Error::RecvError)?
}

struct Connection {
    client: ModbusClient,
    address_offset: i8,
    mqtt: mqtt::Handle,
    shutdown: Shutdown,
    rx: mpsc::Receiver<Command>,
    tx: mpsc::Sender<Command>,
}

#[derive(Debug)]
pub struct Handle {
    tx: mpsc::Sender<Command>,
}

impl Handle {
    pub async fn write_register(&self, address: u16, data: Vec<Word>) -> crate::Result<Vec<Word>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Write(address, data, tx))
            .await
            .map_err(|_| Error::SendError)?;
        rx.await.map_err(|_| Error::RecvError)?
    }
    pub async fn read_input_register(
        &self,
        address: u16,
        quantity: u8,
    ) -> crate::Result<Vec<Word>> {
        self.read_register(ReadType::Input, address, quantity).await
    }
    pub async fn read_holding_register(
        &self,
        address: u16,
        quantity: u8,
    ) -> crate::Result<Vec<Word>> {
        self.read_register(ReadType::Holding, address, quantity)
            .await
    }

    async fn read_register(
        &self,
        reg_type: ReadType,
        address: u16,
        quantity: u8,
    ) -> crate::Result<Vec<Word>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Read(reg_type, address, quantity, tx))
            .await
            .map_err(|_| Error::SendError)?;
        rx.await.map_err(|_| Error::RecvError)?
    }
}

type Word = u16;
type Response = oneshot::Sender<crate::Result<Vec<Word>>>;

#[derive(Clone, Copy, Debug)]
enum ReadType {
    Input,
    Holding,
}

#[derive(Debug)]
enum Command {
    Read(ReadType, u16, u8, Response),
    Write(u16, Vec<Word>, Response),
}

impl Connection {
    pub async fn run(mut self) -> crate::Result<()> {
        let mut input_registers = self.mqtt.subscribe("input/+").await?;
        let mut holding_registers = self.mqtt.subscribe("holding/+").await?;

        // TODO: if we get a new register definition for an existing register, how do we avoid redundant (and possibly
        // conflicting) tasks? Should MQTT component only allow one subscriber per topic filter, replacing the old one
        // when it gets a new subscribe request?

        loop {
            select! {
                Some(reg) = input_registers.recv() => {},
                Some(reg) = holding_registers.recv() => {},
                Some(cmd) = self.rx.recv() => { self.process_command(cmd).await; },
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            }
        }
    }

    fn handle(&self) -> Handle {
        Handle {
            tx: self.tx.clone(),
        }
    }

    /// Apply address offset to address.
    ///
    /// Panics if offset would overflow or underflow the address.
    fn adjust_address(&self, address: u16) -> u16 {
        if self.address_offset.is_zero() {
            return address;
        }

        // TODO: use `checked_add_signed()` once stabilised: https://doc.rust-lang.org/std/primitive.u16.html#method.checked_add_signed
        let adjusted_address = if self.address_offset >= 0 {
            address.checked_add(self.address_offset as u16)
        } else {
            address.checked_sub(self.address_offset.unsigned_abs() as u16)
        };

        if let Some(address) = adjusted_address {
            address
        } else {
            error!(address, offset = self.address_offset,);
            address
            // panic!("Address offset would underflow/overflow")
        }
    }

    #[tracing::instrument(skip(self))]
    async fn process_command(&mut self, cmd: Command) {
        use tokio_modbus::prelude::Reader;

        let (tx, response) = match cmd {
            Command::Read(ReadType::Input, address, count, tx) => {
                let address = self.adjust_address(address);
                (
                    tx,
                    self.client
                        .read_input_registers(address, count as u16)
                        .await,
                )
            }
            Command::Read(ReadType::Holding, address, count, tx) => {
                let address = self.adjust_address(address);
                (
                    tx,
                    self.client
                        .read_holding_registers(address, count as u16)
                        .await,
                )
            }
            Command::Write(address, data, tx) => {
                let address = self.adjust_address(address);
                (
                    tx,
                    self.client
                        .read_write_multiple_registers(
                            address,
                            data.len() as u16,
                            address,
                            &data[..],
                        )
                        .await,
                )
            }
        };

        // This might be transient, so don't kill connection. We may be able to discriminate on the error to determine
        // which errors are transient and which are conclusive.
        //
        // Some errors that we have observed:
        //
        //     Error { kind: UnexpectedEof, message: "failed to fill whole buffer" }'
        //     Custom { kind: InvalidData, error: "Invalid data length: 0" }'
        //     Os { code: 36, kind: Uncategorized, message: "Operation now in progress" }'
        //     Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
        //
        if let Err(error) = &response {
            warn!(?error, "modbus command error");
        }

        // This probably just means that the register task died or is no longer monitoring the response.
        if let Err(response) = tx.send(response.map_err(Into::into)) {
            warn!(?response, "error sending response");
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
