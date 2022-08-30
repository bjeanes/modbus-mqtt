use std::io::Error;
use tokio_modbus::client::Context;
use tokio_modbus::prelude::Client;
use tokio_modbus::slave::Slave;

pub async fn connect<H>(host: H) -> Result<Context, Error>
where
    H: Into<String>,
{
    connect_slave(host, Slave(1)).await
}

pub async fn connect_slave<H>(host: H, slave: Slave) -> Result<Context, Error>
where
    H: Into<String>,
{
    let context = crate::service::connect_slave(host, slave).await?;
    let client: Box<dyn Client> = Box::new(context);
    Ok(Context::from(client))
}
