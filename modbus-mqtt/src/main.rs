use rumqttc::{self, AsyncClient, Event, Incoming, LastWill, MqttOptions, Publish, QoS};
use serde::Serialize;
use serde_json::json;
use std::{collections::HashMap, time::Duration};
use tokio::{sync::mpsc, sync::oneshot, time::MissedTickBehavior};
use tokio_modbus::prelude::*;
use tracing::{debug, error, info};

use clap::Parser;

mod modbus;

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

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum MainStatus {
    Running,
    Stopped,
}

#[tokio::main(worker_threads = 3)]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let (registry_tx, registry_rx) = mpsc::channel::<RegistryCommand>(32);
    let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<DispatchCommand>(32);

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
#[tracing::instrument(level = "debug")]
async fn mqtt_dispatcher(
    mut options: MqttOptions,
    prefix: String,
    registry: mpsc::Sender<RegistryCommand>,
    mut rx: mpsc::Receiver<DispatchCommand>,
) {
    info!("Connecting to MQTT broker...");

    options.set_last_will(LastWill {
        topic: format!("{}/status", prefix),
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
            format!("{}/status", prefix),
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
            info!("Start dispatcher rx loop");
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
            In(Incoming::ConnAck(_)) => info!("Connected to MQTT!"),
            In(Incoming::PingResp | Incoming::SubAck(_)) => (),

            In(Incoming::Publish(Publish { topic, payload, .. })) => {
                debug!("{} -> {:?}", &topic, &payload);

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
                debug!("{:?}", event);
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

#[tracing::instrument(level = "debug")]
async fn connection_registry(
    prefix: String,
    dispatcher: mpsc::Sender<DispatchCommand>,
    mut rx: mpsc::Receiver<RegistryCommand>,
) {
    info!("Starting connection registry...");
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
                info!(id, payload = ?details, "Establishing connection");
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
            _ => error!("unimplemented"),
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

#[tracing::instrument(level = "debug")]
async fn handle_connect(
    dispatcher: mpsc::Sender<DispatchCommand>,
    id: ConnectionId,
    topic_prefix: String,
    payload: bytes::Bytes,
) {
    use modbus::config::*;
    use modbus::ConnectState;
    info!("Starting connection handler for {}", id);
    match serde_json::from_slice::<Connect>(&payload) {
        Ok(connect) => {
            let unit = connect.unit;

            let mut modbus = match connect.settings {
                ModbusProto::SungrowWiNetS { ref host } => {
                    tokio_modbus_winets::connect_slave(host, unit)
                        .await
                        .unwrap()
                }
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
            let status = modbus::ConnectStatus {
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
                                        .read_write_multiple_registers(
                                            address,
                                            data.len() as u16,
                                            address,
                                            &data[..],
                                        )
                                        .await,
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
                        "error": format!("Invalid config: {}", err),
                    }))
                    .unwrap(),
                })
                .await
                .unwrap();
        }
    }
}

#[tracing::instrument(level = "debug")]
async fn watch_registers(
    read_type: ModbusReadType,
    address_offset: i8,
    duration: Duration,
    registers: Vec<modbus::config::Register>,
    modbus: mpsc::Sender<ModbusCommand>,
    dispatcher: mpsc::Sender<DispatchCommand>,
    registers_prefix: String,
) -> ! {
    let mut interval = tokio::time::interval(duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        for r in registers.iter() {
            let address = if address_offset >= 0 {
                r.address.checked_add(address_offset as u16)
            } else {
                r.address.checked_sub(address_offset.unsigned_abs() as u16)
            };
            if let Some(address) = address {
                let size = r.parse.value_type.size();
                debug!(
                    name = r.name.as_ref().unwrap_or(&"".to_string()),
                    address,
                    size,
                    register_type = ?read_type,
                    value_type = r.parse.value_type.type_name(),
                    "Polling register",
                );

                let (tx, rx) = oneshot::channel();

                modbus
                    .send(ModbusCommand::Read(read_type, address, size, tx))
                    .await
                    .unwrap();

                // FIXME: definitely getting errors here that need to be handled
                //
                // thread 'tokio-runtime-worker' panicked at 'called `Result::unwrap()` on an `Err` value: Error { kind: UnexpectedEof, message: "failed to fill whole buffer" }'
                // thread 'tokio-runtime-worker' panicked at 'called `Result::unwrap()` on an `Err` value: Custom { kind: InvalidData, error: "Invalid data length: 0" }'
                // thread 'tokio-runtime-worker' panicked at 'called `Result::unwrap()` on an `Err` value: Os { code: 36, kind: Uncategorized, message: "Operation now in progress" }'
                // thread 'tokio-runtime-worker' panicked at 'called `Result::unwrap()` on an `Err` value: Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
                //
                // Splitting out the two awaits so I can see if all of the above panics come from the same await or some from one vs the other:
                let response = rx.await.unwrap(); // await may have errorer on receiving
                let words = response.unwrap(); // received message is also a result which may be a (presumably Modbus?) error

                let swapped_words = r.apply_swaps(&words);

                let value = r.parse_words(&swapped_words);

                debug!(
                    name = r.name.as_ref().unwrap_or(&"".to_string()),
                    address,
                    %value,
                    raw = ?words,
                    "Received value",
                );

                let payload = serde_json::to_vec(&json!({ "value": value, "raw": words })).unwrap();

                dispatcher
                    .send(DispatchCommand::Publish {
                        topic: format!("{}/{}", registers_prefix, r.address),
                        payload: payload.clone(),
                    })
                    .await
                    .unwrap();

                if let Some(name) = &r.name {
                    dispatcher
                        .send(DispatchCommand::Publish {
                            topic: format!("{}/{}", registers_prefix, name),
                            payload,
                        })
                        .await
                        .unwrap();
                }
            }
        }
    }
}
