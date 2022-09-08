use crate::modbus::{connection, register};
use crate::mqtt::{Payload, Scopable};
use crate::{mqtt, shutdown::Shutdown};
use serde::Deserialize;
use serde_json::value::RawValue as RawJSON;
use tokio::select;
use tracing::{debug, error, info};

/// The topic filter under the prefix to look for connection configs
const TOPIC: &str = "+/connect";

/// Responsible for monitoring MQTT topic for connection configs
pub struct Connector {
    mqtt: mqtt::Handle,
    shutdown: Shutdown,
    // connections: Vec<connection::Handle>,
}

pub(crate) fn new(mqtt: mqtt::Handle, shutdown: Shutdown) -> Connector {
    Connector {
        mqtt,
        shutdown,
        // connections: vec![],
    }
}

impl Connector {
    pub async fn run(&mut self) -> crate::Result<()> {
        let mut new_connection = self.mqtt.subscribe(TOPIC).await?;

        loop {
            select! {
                Some(Payload { bytes, topic }) = new_connection.recv() => {
                    // `unwrap()` is safe here because of the shape of valid topics and the fact that we are subcribed
                    // to a topic under a prefix.
                    let connection_id = topic.rsplit('/').nth_back(1).unwrap();
                    let mqtt = self.mqtt.scoped(connection_id);

                    debug!(?connection_id, ?bytes, ?topic, "Received connection config");

                    if let Err(error) = parse_and_connect(bytes, mqtt, self.shutdown.clone()).await {
                        error!(?connection_id, ?error, "Error creating connection");
                    }

                },

                _ = self.shutdown.recv() => {
                    info!("shutting down connector");
                    break;
                },
            }
        }

        Ok(())
    }
}

async fn parse_and_connect(
    bytes: bytes::Bytes,
    mqtt: mqtt::Handle,
    shutdown: Shutdown,
) -> crate::Result<()> {
    match serde_json::from_slice(&bytes) {
        Err(_) => mqtt.publish("state", "invalid").await?,
        Ok(Config {
            connection:
                connection::Config {
                    settings: connection::ModbusProto::Unknown,
                    ..
                },
            ..
        }) => mqtt.publish("state", "unknown_proto").await?,
        Ok(config) => {
            debug!(?config);
            connect(config, mqtt, shutdown).await?;
        }
    }
    Ok(())
}
async fn connect(config: Config<'_>, mqtt: mqtt::Handle, shutdown: Shutdown) -> crate::Result<()> {
    if shutdown.is_shutdown() {
        return Ok(());
    }

    let Config {
        connection: settings,
        input,
        holding,
    } = config;

    mqtt.publish("state", "connecting").await?;

    // connection isn't able to be Send, so we have to create connection inside its run task and find out if instantiation failed by connection on a channel.
    {
        let mqtt = mqtt.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<crate::Result<()>>();
        tokio::spawn(async move {
            match connection::new(settings, mqtt.clone(), shutdown).await {
                Ok(connection) => {
                    if let Err(e) = mqtt.publish("state", "connected").await {
                        tx.send(Err(e))
                    } else {
                        tx.send(Ok(()))
                    }
                    .expect("unexpected closed receiver");

                    if let Err(error) = connection.run().await {
                        error!(?error, "Modbus connection quit unexpectedly");
                    }
                }
                Err(e) => {
                    tx.send(Err(e)).expect("unexpected closed receiver");
                }
            }
        });

        rx.await.map_err(|_| crate::Error::RecvError)??;
    }

    for reg in input {
        let mqtt = mqtt.scoped("input");
        if let Ok(r) = serde_json::from_slice::<register::AddressedRegister>(reg.get().as_bytes()) {
            debug!(?r);
            let bytes: bytes::Bytes = reg.get().as_bytes().to_owned().into();
            mqtt.publish(r.address.to_string(), bytes).await?;
        }
    }
    for reg in holding {
        let mqtt = mqtt.scoped("holding");
        if let Ok(r) = serde_json::from_slice::<register::AddressedRegister>(reg.get().as_bytes()) {
            debug!(?r);
            let bytes: bytes::Bytes = reg.get().as_bytes().to_owned().into();
            mqtt.publish(r.address.to_string(), bytes).await?;
        }
    }

    Ok(())
}

/// Wrapper around `modbus::connection::Config` that can include some registers inline, which the connector will
/// re-publish to the appropriate topic once the connection is established.
#[derive(Debug, Deserialize)]
struct Config<'a> {
    #[serde(flatten)]
    connection: connection::Config,

    // Allow registers to be defined inline, but capture them as raw JSON so that if they have incorrect schema, we can
    // still establish the Modbus connection. Valid registers will be re-emitted as individual register configs to MQTT,
    // to be picked up by the connection.
    #[serde(default, borrow)]
    pub input: Vec<&'a RawJSON>,
    #[serde(alias = "hold", default, borrow)]
    pub holding: Vec<&'a RawJSON>,
}
