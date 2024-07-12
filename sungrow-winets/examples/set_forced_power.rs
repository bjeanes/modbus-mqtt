use sungrow_winets::*;

// The documented register for setting the charge/discharge power for forced mode is 13052.
//
// HOWEVER, this register can't be set (neither via Modbus nor via WiNet-S register setting). On the other hand, the
// Energy Management Parameters tab lets you set this value, but inspecting the web requests reveals it uses register
// 33148!
//
// This example, therefore, uses register 33148. However, unlike the documented 13052, the value here is set in
// multiples of 10W (e.g. `200` is 2000 Watts).
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let host = std::env::args()
        .nth(1)
        .expect("must pass host/IP of WiNet-S as first argument");

    let power: u16 = str::parse(
        &std::env::args()
            .nth(2)
            .expect("pass power in watts as second argument"),
    )
    .expect("invalid uint");

    let mut client = ClientBuilder::new(host).build()?;
    client.connect().await?;

    let was = client
        .read_register(RegisterType::Holding, 33148, 1)
        .await?;

    println!("power was {} W", 10 * &was[0]);

    client.write_register(33148, &[power / 10]).await?;

    let is = client
        .read_register(RegisterType::Holding, 33148, 1)
        .await?;

    println!("power is now {} W", 10 * &is[0]);

    Ok(())
}
