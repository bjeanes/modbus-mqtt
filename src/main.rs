use rumqttc::{self, AsyncClient, Event, Incoming, LastWill, MqttOptions, Publish, QoS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc;
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

#[derive(Serialize, Deserialize)]
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
        // data_bits: tokio_serial::DataBits, // TODO: allow this to be represented as a number instead of string
        // stop_bits: tokio_serial::StopBits, // TODO: allow this to be represented as a number instead of string
        // flow_control: tokio_se&rial::FlowControl,
        // parity: tokio_serial::Parity,
    },
}

fn default_modbus_port() -> u16 {
    502
}

#[derive(Serialize, Deserialize)]
struct Range {
    address: u16,
    size: u16,
}

// TODO: `scale`, `offset`, `precision`
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
    String,
}

#[derive(Serialize, Deserialize)]
struct RegisterParse {
    #[serde(default = "default_swap")]
    swap_bytes: bool,

    #[serde(default = "default_swap")]
    swap_words: bool,
}

fn default_swap() -> bool {
    false
}

#[derive(Serialize, Deserialize)]
struct Register {
    #[serde(flatten)]
    range: Range,

    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    parse: Option<RegisterParse>,
}

#[derive(Serialize, Deserialize)]
struct Connect {
    #[serde(flatten)]
    settings: ModbusProto,

    // input_ranges: Vec<Register>,
    // hold_ranges: Vec<Register>,
    #[serde(default = "default_modbus_unit")]
    slave: u8, // TODO make `Slave` but need custom deserializer I think

    #[serde(default = "default_address_offset")]
    address_offset: i8,
}

fn default_modbus_unit() -> u8 {
    0
}
fn default_address_offset() -> i8 {
    0
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
        tokio::spawn(async move { connection_registry(prefix, dispatcher_tx, registry_rx).await })
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

        tokio::spawn(async move {
            mqtt_dispatcher(options, prefix, registry_tx, dispatcher_rx).await;
        })
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
                    tokio::spawn(async move {
                        handle_connect(dispatcher, id, prefix, details).await;
                    }),
                );
            }
            _ => println!("unimplemented"),
        }
    }
}

async fn handle_connect(
    dispatcher: mpsc::Sender<DispatchCommand>,
    id: ConnectionId,
    topic_prefix: String,
    payload: bytes::Bytes,
) {
    println!("Starting connection handler for {}", id);
    match serde_json::from_slice::<Connect>(&payload) {
        Ok(connect) => {
            let slave = Slave(connect.slave);
            // println!("{:?}", connect);

            let mut modbus = match connect.settings {
                ModbusProto::Tcp { ref host, port } => {
                    let socket_addr = format!("{}:{}", host, port).parse().unwrap();
                    tcp::connect_slave(socket_addr, slave).await.unwrap()
                }
                ModbusProto::Rtu { ref tty, baud_rate } => {
                    let builder = tokio_serial::new(tty, baud_rate);
                    let port = tokio_serial::SerialStream::open(&builder).unwrap();
                    rtu::connect_slave(port, slave).await.unwrap()
                }
            };
            let status = ConnectStatus {
                connect: connect,
                status: ConnectState::Connected,
            };
            dispatcher
                .send(DispatchCommand::Publish {
                    topic: format!("{}/status/{}", topic_prefix, id),
                    payload: serde_json::to_vec(&status).unwrap(),
                })
                .await
                .unwrap();
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

// async fn requests(client: AsyncClient) {
//     client
//         .subscribe("hello/world", QoS::AtMostOnce)
//         .await
//         .unwrap();

//     for i in 1..=10 {
//         client
//             .publish("hello/world", QoS::ExactlyOnce, false, vec![1; i])
//             .await
//             .unwrap();

//         time::sleep(Duration::from_secs(1)).await;
//     }

//     time::sleep(Duration::from_secs(120)).await;
// }
