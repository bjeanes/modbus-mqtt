use crate::{modbus, mqtt};

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

    // TODO: make sure mqtt connection is last thing to shutdown, so other components can send final messages.
    mqtt_options.set_last_will(rumqttc::LastWill {
        topic: prefix.clone(),
        message: "offline".into(),
        qos: rumqttc::QoS::AtMostOnce,
        retain: false,
    });
    let mut mqtt_connection = mqtt::new(
        mqtt_options,
        (notify_shutdown.subscribe(), shutdown_complete_tx.clone()).into(),
    )
    .await;
    mqtt_connection
        .handle()
        .publish(prefix.clone(), "online")
        .await?;
    let mqtt = mqtt_connection.prefixed_handle(prefix)?;

    let mut connector = modbus::connector::new(
        mqtt.clone(),
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

    timeout(Duration::from_secs(5), shutdown_complete_rx.recv())
        .await
        .map_err(|_| {
            crate::Error::Other("Shutdown didn't complete within 5 seconds; aborting".into())
        })?;

    info!("Shutdown.");

    Ok(())
}
