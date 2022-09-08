use clap::Parser;
use modbus_mqtt::{server, Result};
use rumqttc::MqttOptions;
use url::Url;

#[derive(Parser, Debug)]
#[clap(
    name = "modbus-mqtt",
    version,
    author,
    about = "A bridge between Modbus and MQTT"
)]
struct Cli {
    #[clap(
        env = "MQTT_URL",
        // validator = "is_mqtt_url",
        default_value = "mqtt://localhost:1883/modbus-mqtt",
        value_hint = clap::ValueHint::Url,
        help = "Pass the topic prefix as the URL path"
    )]
    url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let Cli { mut url } = Cli::parse();

    let mut prefix = url
        .path()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_owned();

    let options: MqttOptions = match url.clone().try_into() {
        Ok(options) => options,
        Err(rumqttc::OptionError::ClientId) => {
            let url = url
                .query_pairs_mut()
                .append_pair("client_id", env!("CARGO_PKG_NAME"))
                .finish()
                .clone();
            url.try_into()?
        }
        Err(other) => return Err(other.into()),
    };

    if prefix.is_empty() {
        prefix = options.client_id();
    }

    server::run(prefix, options, tokio::signal::ctrl_c()).await?;

    Ok(())
}
