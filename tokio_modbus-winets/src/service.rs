use std::io::Error;

use tokio_modbus::{
    prelude::{Client, Request, Response},
    slave::{Slave, SlaveContext},
};
use tracing::{debug, error, info};

pub(crate) async fn connect_slave<H>(host: H, _slave: Slave) -> Result<Context, Error>
where
    H: Into<String>,
{
    let host: String = host.into();
    Ok(Context::new(host).await?)
}

#[derive(Debug)]
pub struct Context {
    // unit: Slave,
    host: String,
    service: sungrow_winets::Client,
}

impl Context {
    async fn new(host: String) -> Result<Self, Error> {
        let service = sungrow_winets::Client::new(&host).await?;
        Ok(Self { host, service })
    }
}

#[async_trait::async_trait]
impl Client for Context {
    #[tracing::instrument(level = "debug")]
    async fn call(&mut self, request: Request) -> Result<Response, Error> {
        use sungrow_winets::RegisterType;
        use Request::*;
        match request {
            ReadInputRegisters(address, qty) => {
                let words = self
                    .service
                    .read_register(RegisterType::Input, address, qty)
                    .await?;
                Ok(Response::ReadInputRegisters(words))
            }
            ReadHoldingRegisters(address, qty) => {
                let words = self
                    .service
                    .read_register(RegisterType::Holding, address, qty)
                    .await?;
                Ok(Response::ReadHoldingRegisters(words))
            }
            WriteSingleRegister(address, word) => self
                .call(Request::WriteMultipleRegisters(address, vec![word]))
                .await
                .map(|res| match res {
                    Response::WriteMultipleRegisters(address, _) => {
                        Response::WriteSingleRegister(address, word)
                    }
                    _ => panic!("this should not happen"),
                }),

            WriteMultipleRegisters(address, words) => {
                self.service.write_register(address, &words).await?;
                Ok(Response::WriteMultipleRegisters(
                    address,
                    words.len().try_into().unwrap(),
                ))
            }
            // NOTE: does this notionally read _then_ write or vice versa? If you read the address you are writing, are
            // you supposed to get the old value or the new value?
            ReadWriteMultipleRegisters(read_address, qty, write_address, words) => {
                self.call(Request::WriteMultipleRegisters(write_address, words))
                    .await?;
                self.call(Request::ReadHoldingRegisters(read_address, qty))
                    .await
                    .map(|res| match res {
                        Response::ReadHoldingRegisters(words) => {
                            Response::ReadWriteMultipleRegisters(words)
                        }
                        _ => panic!("this should not happen"),
                    })
            }
            Disconnect => todo!(),
            _ => unimplemented!("Sungrow doesn't use or expose this"),
        }
    }
}

impl SlaveContext for Context {
    // TODO: Technically, the battery is exposed (albeit only in some firmware versions of battery) as another slave on
    // the WiNet-S. However, implementing accessing both will need to be thought about carefully such that the websocket
    // is shared, due to the way the WiNet-S boots off sessions when there are too many accessers.
    // Because the usecase is primarily to access the inverter and most, if not all, battery info is available via the
    // inverter, this is not a priority to implement.
    fn set_slave(&mut self, _slave: tokio_modbus::slave::Slave) {
        unimplemented!()
        // self.unit = slave;
    }
}
