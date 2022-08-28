pub mod winets {
    use async_trait::async_trait;
    use std::io::Error;
    use tokio::time::MissedTickBehavior;
    use tokio_modbus::client::Client;
    use tokio_modbus::client::Context as ModbusContext;
    use tokio_modbus::prelude::{Request, Response};
    use tokio_modbus::slave::{Slave, SlaveContext};

    use tracing::{debug, error, info};

    pub async fn connect<H>(host: H) -> Result<ModbusContext, Error>
    where
        H: Into<String>,
    {
        connect_slave(host, Slave(1)).await
    }

    pub async fn connect_slave<H>(host: H, slave: Slave) -> Result<ModbusContext, Error>
    where
        H: Into<String>,
    {
        let (tx, mut rx) = tokio::sync::watch::channel(None);

        tokio::spawn(async move {
            debug!("Starting WiNet-S websocket");
            use futures_util::SinkExt;
            // use futures_util::{future, pin_mut, StreamExt};
            use futures_util::StreamExt;
            use std::time::Duration;
            // use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use serde_json::Value as JSON;
            use tokio::select;
            use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

            let ws_url = format!("ws://{}:8082/ws/home/overview", "10.10.10.219");
            let (mut ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");
            // let (write, read) = ws_stream.split();
            ws_stream
                .send(Message::Text(
                    serde_json::json!({"lang":"en_us","token":"","service":"connect"}).to_string(),
                ))
                .await
                .expect("whoops");

            // WiNet-S interface sends following message every now and then:
            //   {"lang":"zh_cn","service":"ping","token":"","id":"84c2265b-5f7f-4915-82e9-57250064316f"}
            // UUID is always random, token always seems blank.
            // Unclear if this is a real `Ping` message or just a regular `Text` message with "ping" content.
            //  update: it is just a text message 🙄
            // Response is just:
            //   { "result_code":	1, "result_msg":	"success" }
            let mut ping = tokio::time::interval(Duration::from_secs(5));
            ping.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                select! {
                    Some(resp) = ws_stream.next() => {
                        match resp {
                            Ok(msg) => {
                                debug!(%msg, "WS ->");

                                if let Message::Text(msg) = msg {
                                    let value: JSON =  serde_json::from_str(&msg).expect("expected json");
                                    if let JSON::String(ref token) = value["result_data"]["token"] {
                                        // FIXME: this should fails when all receivers have been dropped but I'm pretty
                                        // sure rx is not dropped because it's moved into Context struct :/
                                        tx.send(Some(token.clone())).unwrap();
                                    }
                                }
                            },
                            Err(err) => error!(?err, "WS ->")
                        }
                    },
                    _ = ping.tick() => {
                        let msg = serde_json::json!({
                            "lang":"en_us", // WiNet-S always sends zh_cn, but this works
                            "service":"ping",
                            // WiNet-S includes `"token": ""`, but it works without it
                            "id": uuid::Uuid::new_v4()
                        }).to_string();
                        debug!(%msg, "WS <-");
                        ws_stream
                            .send(Message::Text(msg))
                            .await
                            .expect("whoops");
                    }
                }
            }
        });

        // wait for a token before returning the client, so that it is ready
        rx.changed().await;

        let box_: Box<dyn Client> = Box::new(Context {
            unit: Some(slave),
            token: rx,
        });
        Ok(ModbusContext::from(box_))
    }

    /// Equivalent to tokio_modbus::service::tcp::Context
    #[derive(Debug)]
    pub struct Context {
        unit: Option<crate::modbus::Unit>,
        token: tokio::sync::watch::Receiver<Option<String>>,
        // TODO: websocket + keep TCP connection for HTTP?
    }

    #[async_trait]
    impl Client for Context {
        #[tracing::instrument(level = "debug")]
        async fn call(&mut self, request: Request) -> Result<Response, Error> {
            match request {
                Request::ReadCoils(_, _) => todo!(),
                Request::ReadDiscreteInputs(_, _) => todo!(),
                Request::WriteSingleCoil(_, _) => todo!(),
                Request::WriteMultipleCoils(_, _) => todo!(),
                Request::ReadInputRegisters(_, _) => {
                    Result::Ok(Response::ReadInputRegisters(vec![0xaa]))
                }
                Request::ReadHoldingRegisters(_, _) => todo!(),
                Request::WriteSingleRegister(_, _) => todo!(),
                Request::WriteMultipleRegisters(_, _) => todo!(),
                Request::ReadWriteMultipleRegisters(_, _, _, _) => todo!(),
                Request::Custom(_, _) => todo!(),
                Request::Disconnect => todo!(),
            }
        }
    }

    impl SlaveContext for Context {
        fn set_slave(&mut self, slave: tokio_modbus::slave::Slave) {
            self.unit = Some(slave);
        }
    }
}
