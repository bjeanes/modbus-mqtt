use rumqttc::{self, AsyncClient, Event, Incoming, LastWill, MqttOptions, Publish, QoS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, time::Duration};
use tokio::{sync::mpsc, sync::oneshot, time::MissedTickBehavior};
use tokio_modbus::prelude::*;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    mqtt_host: String,

    #[clap(short = 'n', long, default_value = "modbus")]
    mqtt_name: String,

    #[clap(short = 'p', long, default_value_t = 1883)]
    mqtt_port: u16,

    #[clap(short = 'u', long, env = "MQTT_USER")]
    mqtt_user: Option<String>,

    #[clap(short = 'P', long, env)]
    mqtt_password: Option<String>,

    #[clap(short = 't', long, default_value = "modbus-mqtt")]
    // Where to listen for commands
    mqtt_topic_prefix: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ModbusProto {
    Tcp {
        host: String,

        #[serde(default = "default_modbus_port")]
        port: u16,
    },
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
}

fn default_modbus_port() -> u16 {
    502
}

fn default_modbus_data_bits() -> tokio_serial::DataBits {
    tokio_serial::DataBits::Eight
}

fn default_modbus_stop_bits() -> tokio_serial::StopBits {
    tokio_serial::StopBits::One
}

fn default_modbus_flow_control() -> tokio_serial::FlowControl {
    tokio_serial::FlowControl::None
}

fn default_modbus_parity() -> tokio_serial::Parity {
    tokio_serial::Parity::None
}

// TODO: `scale`, `offset`, `precision`
// TODO: migrate `count` from `Range` into this enum to force the correct size?
#[derive(Clone, Serialize, Deserialize)]
enum RegisterValueType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    // Array(u16, RegisterValueType),
    String(u16),
}

#[derive(Clone, Serialize, Deserialize)]
struct RegisterParse {
    #[serde(default = "default_swap")]
    swap_bytes: bool,

    #[serde(default = "default_swap")]
    swap_words: bool,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    value_type: Option<RegisterValueType>,
}

fn default_swap() -> bool {
    false
}

#[derive(Clone, Serialize, Deserialize)]
struct Range {
    address: u16,

    #[serde(alias = "size")]
    count: u8, // Modbus limits to 125 in fact - https://github.com/slowtec/tokio-modbus/issues/112#issuecomment-1095316069=
}

#[derive(Clone, Serialize, Deserialize)]
struct Register {
    #[serde(flatten)]
    range: Range,

    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    parse: Option<RegisterParse>,

    #[serde(
        with = "humantime_serde",
        default = "default_register_interval",
        alias = "period",
        alias = "duration"
    )]
    interval: Duration,
}

fn default_register_interval() -> Duration {
    Duration::from_secs(10)
}

#[derive(Clone, Serialize, Deserialize)]
struct Connect {
    #[serde(flatten)]
    settings: ModbusProto,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    input: Vec<Register>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hold: Vec<Register>,

    #[serde(alias = "slave", default = "default_modbus_unit", with = "ext::Unit")]
    unit: Unit,

    #[serde(default = "default_address_offset")]
    address_offset: i8,
}

fn default_modbus_unit() -> Unit {
    Slave(0)
}
fn default_address_offset() -> i8 {
    0
}

type UnitId = SlaveId;
type Unit = Slave;
mod ext {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    #[serde(remote = "tokio_modbus::slave::Slave")]
    pub struct Unit(pub crate::UnitId);
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum ConnectState {
    Connected,
    Disconnected,
    Errored,
}

#[derive(Serialize)]
struct ConnectStatus {
    #[serde(flatten)]
    connect: Connect,
    status: ConnectState,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum MainStatus {
    Running,
    Stopped,
}

#[tokio::main(worker_threads = 1)]
async fn main() {
    let args = Cli::parse();

    let (registry_tx, mut registry_rx) = mpsc::channel::<RegistryCommand>(32);
    let (dispatcher_tx, mut dispatcher_rx) = mpsc::channel::<DispatchCommand>(32);

    // Modbus connection registry
    let registry_handle = {
        let prefix = args.mqtt_topic_prefix.clone();
        tokio::spawn(connection_registry(prefix, dispatcher_tx, registry_rx))
    };

    // MQTT Dispatcher
    let dispatcher_handle = {
        let prefix = args.mqtt_topic_prefix.clone();
        let mut options = MqttOptions::new(
            env!("CARGO_PKG_NAME"),
            args.mqtt_host.as_str(),
            args.mqtt_port,
        );
        if let (Some(u), Some(p)) = (args.mqtt_user, args.mqtt_password) {
            options.set_credentials(u, p);
        }
        options.set_keep_alive(Duration::from_secs(5)); // TODO: make this configurable

        tokio::spawn(mqtt_dispatcher(options, prefix, registry_tx, dispatcher_rx))
    };

    registry_handle.await.unwrap();
    dispatcher_handle.await.unwrap();
}

#[derive(Debug)]
enum DispatchCommand {
    Publish { topic: String, payload: Vec<u8> },
}
async fn mqtt_dispatcher(
    mut options: MqttOptions,
    prefix: String,
    registry: mpsc::Sender<RegistryCommand>,
    mut rx: mpsc::Receiver<DispatchCommand>,
) {
    println!("Connecting to MQTT broker...");

    options.set_last_will(LastWill {
        topic: format!("{}/status", prefix).to_string(),
        message: serde_json::to_vec(&json!({
            "status": MainStatus::Stopped,
        }))
        .unwrap()
        .into(),
        qos: QoS::AtMostOnce,
        retain: false,
    });

    let (client, mut eventloop) = AsyncClient::new(options, 10);

    client
        .publish(
            format!("{}/status", prefix).to_string(),
            QoS::AtMostOnce,
            false,
            serde_json::to_vec(&json!({
                "status": MainStatus::Running,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    client
        .subscribe(format!("{}/connect/#", prefix), QoS::AtMostOnce)
        .await
        .unwrap();

    let rx_loop_handler = {
        let client = client.clone();
        tokio::spawn(async move {
            println!("Start dispatcher rx loop");
            while let Some(command) = rx.recv().await {
                match command {
                    DispatchCommand::Publish { topic, payload } => {
                        client
                            .publish(topic, QoS::AtMostOnce, false, payload)
                            .await
                            .unwrap();
                    }
                }
            }
        })
    };

    while let Ok(event) = eventloop.poll().await {
        use Event::{Incoming as In, Outgoing as Out};

        match event {
            Out(_) => (),
            In(Incoming::ConnAck(_)) => println!("Connected to MQTT!"),
            In(Incoming::PingResp | Incoming::SubAck(_)) => (),

            In(Incoming::Publish(Publish { topic, payload, .. })) => {
                println!("{} -> {:?}", &topic, &payload);

                match topic.split('/').collect::<Vec<&str>>()[..] {
                    [p, "connect", conn_name] if p == prefix.as_str() => {
                        registry
                            .send(RegistryCommand::Connect {
                                id: conn_name.to_string(),
                                details: payload,
                            })
                            .await
                            .unwrap();
                    }
                    _ => (),
                };
            }
            _ => {
                println!("{:?}", event);
            }
        }
    }

    rx_loop_handler.await.unwrap();
}

type ConnectionId = String;

#[derive(Debug)]
enum RegistryCommand {
    Connect {
        id: ConnectionId,
        details: bytes::Bytes,
    },
    Disconnect(ConnectionId),
}

type RegistryDb = HashMap<ConnectionId, tokio::task::JoinHandle<()>>;

async fn connection_registry(
    prefix: String,
    dispatcher: mpsc::Sender<DispatchCommand>,
    mut rx: mpsc::Receiver<RegistryCommand>,
) {
    println!("Starting connection registry...");
    let mut db: RegistryDb = HashMap::new();

    while let Some(command) = rx.recv().await {
        use RegistryCommand::*;
        match command {
            Disconnect(id) => {
                if let Some(handle) = db.remove(&id) {
                    handle.abort();
                }
            }
            Connect { id, details } => {
                println!("Connection {}: {:?}", id, &details);
                let prefix = prefix.clone();
                let dispatcher = dispatcher.clone();

                if let Some(handle) = db.remove(&id) {
                    handle.abort();
                }

                db.insert(
                    id.clone(),
                    tokio::spawn(handle_connect(dispatcher, id, prefix, details)),
                );
            }
            _ => println!("unimplemented"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ModbusReadType {
    Input,
    Hold,
}

#[derive(Debug)]
enum ModbusCommand {
    Read(ModbusReadType, u16, u8, ModbusResponse),
    Write(u16, Vec<u16>, ModbusResponse),
}

type ModbusResponse = oneshot::Sender<Result<Vec<u16>, std::io::Error>>;

async fn handle_connect(
    dispatcher: mpsc::Sender<DispatchCommand>,
    id: ConnectionId,
    topic_prefix: String,
    payload: bytes::Bytes,
) {
    println!("Starting connection handler for {}", id);
    match serde_json::from_slice::<Connect>(&payload) {
        Ok(connect) => {
            let unit = connect.unit;

            let mut modbus = match connect.settings {
                ModbusProto::Tcp { ref host, port } => {
                    let socket_addr = format!("{}:{}", host, port).parse().unwrap();
                    tcp::connect_slave(socket_addr, unit).await.unwrap()
                }
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
                    let port = tokio_serial::SerialStream::open(&builder).unwrap();
                    rtu::connect_slave(port, unit).await.unwrap()
                }
            };
            let status = ConnectStatus {
                connect: connect.clone(),
                status: ConnectState::Connected,
            };
            dispatcher
                .send(DispatchCommand::Publish {
                    topic: format!("{}/status/{}", topic_prefix, id),
                    payload: serde_json::to_vec(&status).unwrap(),
                })
                .await
                .unwrap();

            let (modbus_tx, mut modbus_rx) = mpsc::channel::<ModbusCommand>(32);
            tokio::spawn(async move {
                while let Some(command) = modbus_rx.recv().await {
                    match command {
                        ModbusCommand::Read(read_type, address, count, responder) => {
                            let response = match read_type {
                                ModbusReadType::Input => {
                                    modbus.read_input_registers(address, count as u16)
                                }
                                ModbusReadType::Hold => {
                                    modbus.read_holding_registers(address, count as u16)
                                }
                            };

                            responder.send(response.await).unwrap();
                        }
                        ModbusCommand::Write(address, data, responder) => {
                            responder
                                .send(
                                    modbus
                                        .write_multiple_registers(address, &data[..])
                                        .await
                                        .map(|_| vec![]),
                                )
                                .unwrap();
                        }
                    }
                }
            });

            use itertools::Itertools;
            for (duration, registers) in &connect.input.into_iter().group_by(|r| r.interval) {
                let registers_prefix = format!("{}/input/{}", topic_prefix, id);

                tokio::spawn(watch_registers(
                    ModbusReadType::Input,
                    connect.address_offset,
                    duration,
                    registers.collect(),
                    modbus_tx.clone(),
                    dispatcher.clone(),
                    registers_prefix,
                ));
            }
            for (duration, registers) in &connect.hold.into_iter().group_by(|r| r.interval) {
                let registers_prefix = format!("{}/hold/{}", topic_prefix, id);

                tokio::spawn(watch_registers(
                    ModbusReadType::Hold,
                    connect.address_offset,
                    duration,
                    registers.collect(),
                    modbus_tx.clone(),
                    dispatcher.clone(),
                    registers_prefix,
                ));
            }
        }
        Err(err) => {
            dispatcher
                .send(DispatchCommand::Publish {
                    topic: format!("{}/status/{}", topic_prefix, id),
                    payload: serde_json::to_vec(&json!({
                        "status": ConnectState::Errored,
                        "error": format!("Invalid config: {}", err.to_string()),
                    }))
                    .unwrap(),
                })
                .await
                .unwrap();
        }
    }
}

async fn watch_registers(
    read_type: ModbusReadType,
    address_offset: i8,
    duration: Duration,
    registers: Vec<Register>,
    modbus: mpsc::Sender<ModbusCommand>,
    dispatcher: mpsc::Sender<DispatchCommand>,
    registers_prefix: String,
) {
    let mut interval = tokio::time::interval(duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        for ref r in registers.iter() {
            let address = if address_offset >= 0 {
                r.range.address.checked_add(address_offset as u16)
            } else {
                r.range
                    .address
                    .checked_sub(address_offset.unsigned_abs() as u16)
            };
            if let Some(address) = address {
                println!("Polling {:?} {}", read_type, address);

                let (tx, rx) = oneshot::channel();

                modbus
                    .send(ModbusCommand::Read(
                        read_type,
                        address,
                        r.range.count.into(),
                        tx,
                    ))
                    .await
                    .unwrap();

                let values = rx.await.unwrap().unwrap();

                let payload = serde_json::to_vec(&json!({ "raw": values, })).unwrap();

                dispatcher
                    .send(DispatchCommand::Publish {
                        topic: format!("{}/{}", registers_prefix, r.range.address),
                        payload: payload.clone(),
                    })
                    .await
                    .unwrap();

                if let Some(name) = &r.name {
                    dispatcher
                        .send(DispatchCommand::Publish {
                            topic: format!("{}/{}", registers_prefix, name),
                            payload: payload,
                        })
                        .await
                        .unwrap();
                }
            }
        }
    }
}
