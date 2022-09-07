use clap::Parser;
use modbus_mqtt::{server, Result};
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
        value_hint = clap::ValueHint::Url
    )]
    url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut args = Cli::parse();

    let prefix = args
        .url
        .path()
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or(env!("CARGO_PKG_NAME"))
        .to_owned();

    // FIXME: if they pass "?client_id=foo" param, skip this
    args.url
        .query_pairs_mut()
        .append_pair("client_id", env!("CARGO_PKG_NAME"))
        .finish();

    server::run(prefix, args.url.try_into()?, tokio::signal::ctrl_c()).await?;

    Ok(())
}
