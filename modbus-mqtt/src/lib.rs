mod shutdown;

pub mod modbus;
pub mod mqtt;
pub mod server;

//TODO:
// pub mod homeassistant;

mod error;
pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

//                 let (tx, rx) = oneshot::channel();

//                 modbus
//                     .send(ModbusCommand::Read(read_type, address, size, tx))
//                     .await
//                     .unwrap();

//                 // FIXME: definitely getting errors here that need to be handled
//                 //
//                 //
//                 // Splitting out the two awaits so I can see if all of the above panics come from the same await or some from one vs the other:
//                 let response = rx.await.unwrap(); // await may have errorer on receiving
//                 let words = response.unwrap(); // received message is also a result which may be a (presumably Modbus?) error

//                 let swapped_words = r.apply_swaps(&words);

//                 let value = r.parse_words(&swapped_words);

//                 debug!(
//                     name = r.name.as_ref().unwrap_or(&"".to_string()),
//                     address,
//                     %value,
//                     raw = ?words,
//                     "Received value",
//                 );

//                 let payload = serde_json::to_vec(&json!({ "value": value, "raw": words })).unwrap();

//                 dispatcher
//                     .send(DispatchCommand::Publish {
//                         topic: format!("{}/{}", registers_prefix, r.address),
//                         payload: payload.clone(),
//                     })
//                     .await
//                     .unwrap();

//                 if let Some(name) = &r.name {
//                     dispatcher
//                         .send(DispatchCommand::Publish {
//                             topic: format!("{}/{}", registers_prefix, name),
//                             payload,
//                         })
//                         .await
//                         .unwrap();
//                 }
//             }
//         }
//     }
// }
