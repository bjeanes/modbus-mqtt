use crate::{
    modbus,
    mqtt::{self, Scopable},
};

use rumqttc::MqttOptions;
use std::{future::Future, time::Duration};
use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};
use tracing::{error, info};

pub async fn run<P: Into<String> + Send>(
    prefix: P,
    mut mqtt_options: MqttOptions,
    shutdown: impl Future,
) -> crate::Result<()> {
    let prefix = prefix.into();

    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

    mqtt_options.set_last_will(rumqttc::LastWill {
        topic: prefix.clone(),
        message: "offline".into(),
        qos: rumqttc::QoS::AtMostOnce,
        retain: false,
    });
    let mut mqtt_connection = mqtt::new(mqtt_options).await;
    let mqtt = mqtt_connection.handle();
    mqtt.publish(prefix.clone(), "online").await?;

    let mut connector = modbus::connector::new(
        mqtt.scoped(prefix),
        (notify_shutdown.subscribe(), shutdown_complete_tx.clone()).into(),
    );

    tokio::spawn(async move {
        if let Err(err) = mqtt_connection.run().await {
            error!(cause = %err, "MQTT connection error");
        }
    });

    tokio::spawn(async move {
        if let Err(err) = connector.run().await {
            error!(cause = %err, "Modbus connector error");
        }
    });

    shutdown.await;
    drop(notify_shutdown);
    drop(shutdown_complete_tx);

    // We want MQTT to be the last thing to shutdown, so it gets shutdown after everything else
    shutdown_complete_rx.recv().await;
    mqtt.shutdown().await?;

    Ok(())
}
