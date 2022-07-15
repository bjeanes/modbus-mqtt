use rumqttc::{self, AsyncClient, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use tokio::{task, time};

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
}

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let mut mqttoptions = MqttOptions::new("mqtt", args.mqtt_host.as_str(), args.mqtt_port);
    if let (Some(u), Some(p)) = (args.mqtt_user, args.mqtt_password) {
        mqttoptions.set_credentials(u, p);
    }
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    loop {
        let event = eventloop.poll().await;
        println!("{:?}", event.unwrap());
    }
}

async fn requests(client: AsyncClient) {
    client
        .subscribe("hello/world", QoS::AtMostOnce)
        .await
        .unwrap();

    for i in 1..=10 {
        client
            .publish("hello/world", QoS::ExactlyOnce, false, vec![1; i])
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }

    time::sleep(Duration::from_secs(120)).await;
}
