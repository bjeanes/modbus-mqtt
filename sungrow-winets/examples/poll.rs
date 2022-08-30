use std::time::Duration;

use sungrow_winets::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let host = std::env::args()
        .nth(1)
        .expect("must pass host/IP of WiNet-S as first argument");

    let client = Client::new(host).await?;

    let mut tick = tokio::time::interval(Duration::from_millis(200));
    loop {
        tick.tick().await;
        let data = client.running_state().await;
        println!("{:?}", &data);
    }
}
