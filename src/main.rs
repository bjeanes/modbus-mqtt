use rumqttc::{self, AsyncClient, Event, Incoming, LastWill, MqttOptions, Publish, QoS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
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
    Rtu {
        tty: String,
        baud_rate: u32,
    },
}

fn default_modbus_port() -> u16 {
    502
}

#[derive(Serialize, Deserialize)]
struct Connect {
    #[serde(flatten)]
    settings: ModbusProto,

    #[serde(default = "default_modbus_unit")]
    slave: u8, // TODO make `Slave` but need custom deserializer I think
}

fn default_modbus_unit() -> u8 {
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

    let mut mqttoptions = MqttOptions::new("mqtt", args.mqtt_host.as_str(), args.mqtt_port);
    if let (Some(u), Some(p)) = (args.mqtt_user, args.mqtt_password) {
        mqttoptions.set_credentials(u, p);
    }
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_last_will(LastWill {
        topic: format!("{}/status", args.mqtt_topic_prefix).to_string(),
        message: serde_json::to_vec(&json!({
            "status": MainStatus::Stopped,
        }))
        .unwrap()
        .into(),
        qos: QoS::AtMostOnce,
        retain: false,
    });

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client
        .subscribe(
            format!("{}/connect/#", args.mqtt_topic_prefix),
            QoS::AtMostOnce,
        )
        .await
        .unwrap();

    client
        .publish(
            format!("{}/status", args.mqtt_topic_prefix).to_string(),
            QoS::AtMostOnce,
            false,
            serde_json::to_vec(&json!({
                "status": MainStatus::Running,
            }))
            .unwrap(),
        )
        .await
        .unwrap();

    while let Ok(event) = eventloop.poll().await {
        match event {
            Event::Outgoing(_) => (),
            Event::Incoming(Incoming::ConnAck(_)) => println!("Connected to MQTT!"),
            Event::Incoming(Incoming::PingResp | Incoming::SubAck(_)) => (),

            Event::Incoming(Incoming::Publish(Publish { topic, payload, .. })) => {
                println!("{} {:?}", &topic, &payload);
                match topic.split('/').collect::<Vec<&str>>()[..] {
                    [prefix, "connect", conn_name] if prefix == args.mqtt_topic_prefix.as_str() => {
                        match serde_json::from_slice::<Connect>(&payload) {
                            Ok(connect) => {
                                let slave = Slave(connect.slave);
                                // println!("{:?}", connect);
                                let status = match connect.settings {
                                    ModbusProto::Tcp { ref host, port } => {
                                        let socket_addr =
                                            format!("{}:{}", host, port).parse().unwrap();
                                        let mut modbus =
                                            tcp::connect_slave(socket_addr, slave).await.unwrap();
                                        ConnectStatus {
                                            connect: connect,
                                            status: ConnectState::Connected,
                                        }
                                    }
                                    ModbusProto::Rtu { ref tty, baud_rate } => {
                                        let builder = tokio_serial::new(tty, baud_rate);
                                        let port =
                                            tokio_serial::SerialStream::open(&builder).unwrap();
                                        let mut modbus =
                                            rtu::connect_slave(port, slave).await.unwrap();
                                        ConnectStatus {
                                            connect: connect,
                                            status: ConnectState::Connected,
                                        }
                                    }
                                };
                                client
                                    .publish(
                                        format!("{}/status/{}", args.mqtt_topic_prefix, conn_name)
                                            .as_str(),
                                        QoS::AtMostOnce,
                                        false,
                                        serde_json::to_vec(&status).unwrap(),
                                    )
                                    .await
                                    .unwrap();
                            }
                            Err(err) => {
                                client
                                    .publish(
                                        format!("{}/status/{}", args.mqtt_topic_prefix, conn_name)
                                            .as_str(),
                                        QoS::AtMostOnce,
                                        false,
                                        serde_json::to_vec(&json!({
                                            "status": ConnectState::Errored,
                                            "error": err.to_string(),
                                        }))
                                        .unwrap(),
                                    )
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                    _ => (),
                };
            }
            _ => {
                println!("{:?}", event);
            }
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
