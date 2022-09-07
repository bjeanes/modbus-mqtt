use crate::mqtt;
use rumqttc::MqttOptions;
use std::future::Future;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

pub struct Server {
    notify_shutdown: broadcast::Sender<()>,
    mqtt_connection: mqtt::Connection,
}

pub async fn run<P: Into<String>>(
    prefix: P,
    mqtt_options: MqttOptions,
    shutdown: impl Future,
) -> crate::Result<()> {
    let (notify_shutdown, _) = broadcast::channel(1);
    let mqtt_connection = mqtt::new(mqtt_options, notify_shutdown.subscribe().into()).await;

    let mut server = Server {
        notify_shutdown,
        mqtt_connection,
    };

    let mut ret = Ok(());

    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                error!(cause = %err, "server error");
                ret = Err(err)
            } else {
                info!("server finished running")
            }
        }

        _ = shutdown => {
            info!("shutting down");
        }
    }

    let Server {
        notify_shutdown, ..
    } = server;

    drop(notify_shutdown);

    ret
}

impl Server {
    async fn run(&mut self) -> crate::Result<()> {
        info!("Starting up");

        let tx = self.mqtt_connection.prefixed_handle("hello")?;

        {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    tx.send(mqtt::Message::Publish(rumqttc::Publish::new(
                        "foo/bar/baz",
                        rumqttc::QoS::AtLeastOnce,
                        "hello",
                    )))
                    .await
                    .unwrap();
                }
            });
        }

        tokio::spawn(async move {
            let (tx_bytes, mut rx) = tokio::sync::mpsc::channel(32);
            tx.send(mqtt::Message::Subscribe(
                rumqttc::Subscribe::new("foo/+/baz", rumqttc::QoS::AtLeastOnce),
                tx_bytes,
            ))
            .await
            .unwrap();

            while let Some(bytes) = rx.recv().await {
                debug!(?bytes, "received");
            }
        });

        self.mqtt_connection.run().await
    }
}
