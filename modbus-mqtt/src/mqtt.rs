use std::collections::HashMap;

use bytes::Bytes;
use rumqttc::{
    mqttbytes::matches as matches_topic, AsyncClient, Event, EventLoop, MqttOptions, Publish,
    Subscribe, SubscribeFilter,
};
use tokio::{
    select,
    sync::mpsc::{self, channel, Receiver, Sender},
};
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct Payload {
    pub bytes: Bytes,
    pub topic: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Subscribe(Subscribe, Sender<Payload>),
    Publish(Publish),
    Shutdown,
}

pub(crate) async fn new(options: MqttOptions) -> Connection {
    let (client, event_loop) = AsyncClient::new(options, 32);

    let (tx, rx) = channel(32);
    Connection {
        client,
        event_loop,
        subscriptions: HashMap::new(),
        tx,
        rx,
    }
}

// Maintain internal subscriptions as well as MQTT subscriptions. Relay all received messages on MQTT subscribed topics
// to internal components who have a matching topic. Unsubscribe topics when no one is listening anymore.
pub(crate) struct Connection {
    subscriptions: HashMap<String, Vec<Sender<Payload>>>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    client: AsyncClient,
    event_loop: EventLoop,
}

impl Connection {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            select! {
                event = self.event_loop.poll() => {
                    self.handle_event(event?).await?
                }
                request = self.rx.recv() => {
                    match request {
                        None => return Ok(()),
                        Some(Message::Shutdown) => {
                            info!("MQTT connection shutting down");
                            break;
                        }
                        Some(req) => self.handle_request(req).await?,
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle(&self) -> Handle {
        Handle {
            prefix: None,
            tx: self.tx.clone(),
        }
    }

    async fn handle_event(&mut self, event: Event) -> crate::Result<()> {
        use rumqttc::Incoming;

        #[allow(clippy::single_match)]
        match event {
            Event::Incoming(Incoming::Publish(Publish { topic, payload, .. })) => {
                debug!(%topic, ?payload, "publish");
                self.handle_data(topic, payload).await?;
            }
            // e => debug!(event = ?e),
            _ => {}
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(subscriptions = ?self.subscriptions.keys()))]
    async fn handle_data(&mut self, topic: String, bytes: Bytes) -> crate::Result<()> {
        let mut targets = vec![];

        // Remove subscriptions whose channels are closed, adding matching channels to the `targets` vec.
        self.subscriptions.retain(|filter, channels| {
            if matches_topic(&topic, filter) {
                channels.retain(|channel| {
                    if channel.is_closed() {
                        warn!(?channel, "closed");
                        false
                    } else {
                        targets.push(channel.clone());
                        true
                    }
                });
                !channels.is_empty()
            } else {
                true
            }
        });

        for target in targets {
            if target
                .send(Payload {
                    topic: topic.clone(),
                    bytes: bytes.clone(),
                })
                .await
                .is_err()
            {
                // These will be removed above next time a matching payload is removed
            }
        }
        Ok(())
    }

    async fn handle_request(&mut self, request: Message) -> crate::Result<()> {
        match request {
            Message::Publish(Publish {
                topic,
                payload,
                qos,
                retain,
                ..
            }) => {
                self.client
                    .publish_bytes(topic, qos, retain, payload)
                    .await?
            }
            Message::Subscribe(Subscribe { filters, .. }, channel) => {
                for filter in &filters {
                    let channel = channel.clone();

                    // NOTE: Curently allows multiple components to watch the same topic filter, but if there is no need
                    // for this, it might make more sense to have it _replace_ the channel, so that old (stale)
                    // components automatically finish running.
                    match self.subscriptions.get_mut(&filter.path) {
                        Some(channels) => channels.push(channel),
                        None => {
                            self.subscriptions
                                .insert(filter.path.clone(), vec![channel]);
                        }
                    }
                }

                self.client.subscribe_many(filters).await?
            }
            Message::Shutdown => panic!("Handled by the caller"),
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Handle {
    prefix: Option<String>,
    tx: Sender<Message>,
}

// IDEA: make subscribe+publish _generic_ over the payload type, as long as it implements a Payload trait we define,
// which allows them to perform the serialization/deserialization to Bytes. For most domain types, the trait would be
// implemented to use serde_json but for Bytes and Vec<u8> it would just return itself.
// The return values may need to be crate::Result<Receiver<Option<T>> or crate::Result<Receiver<crate::Result<T>>>.
impl Handle {
    pub async fn subscribe<S: Into<String>>(&self, topic: S) -> crate::Result<Receiver<Payload>> {
        let (tx_bytes, rx) = mpsc::channel(8);
        let mut msg =
            Message::Subscribe(Subscribe::new(topic, rumqttc::QoS::AtLeastOnce), tx_bytes);
        if let Some(prefix) = &self.prefix {
            msg = msg.scoped(prefix.to_owned());
        }
        self.tx
            .send(msg)
            .await
            .map_err(|_| crate::Error::SendError)?;
        Ok(rx)
    }
    pub async fn publish<S: Into<String>, B: Into<Bytes>>(
        &self,
        topic: S,
        payload: B,
    ) -> crate::Result<()> {
        let mut msg = Message::Publish(Publish::new(
            topic,
            rumqttc::QoS::AtLeastOnce,
            payload.into(),
        ));
        if let Some(prefix) = &self.prefix {
            msg = msg.scoped(prefix.to_owned());
        }
        self.tx
            .send(msg)
            .await
            .map_err(|_| crate::Error::SendError)?;
        Ok(())
    }

    pub async fn shutdown(self) -> crate::Result<()> {
        self.tx
            .send(Message::Shutdown)
            .await
            .map_err(|_| crate::Error::SendError)
    }
}

pub(crate) trait Scopable {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self;
}

// FIXME: this doesn't actually _prefix_ it _appends_ to the existing prefix, so there's probably a better name for this
// trait, like: Scopable
impl Scopable for Handle {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self {
        match self {
            Self { prefix: None, tx } => Self {
                prefix: Some(prefix.into()),
                tx: tx.clone(),
            },
            Self {
                prefix: Some(existing),
                tx,
            } => Self {
                prefix: Some(format!("{}/{}", existing, prefix.into())),
                tx: tx.clone(),
            },
        }
    }
}

impl Scopable for Message {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self {
        match self {
            Message::Subscribe(sub, bytes) => Message::Subscribe(sub.scoped(prefix), bytes.clone()),
            Message::Publish(publish) => Message::Publish(publish.scoped(prefix)),
            other => (*other).clone(),
        }
    }
}

impl Scopable for Subscribe {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self {
        let prefix: String = prefix.into();
        Self {
            pkid: self.pkid,
            filters: self
                .filters
                .iter()
                .map(|f| f.clone().scoped(prefix.clone()))
                .collect(),
        }
    }
}

impl Scopable for Publish {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self {
        let mut prefixed = self.clone();
        prefixed.topic = format!("{}/{}", prefix.into(), &self.topic);
        prefixed
    }
}

impl Scopable for SubscribeFilter {
    fn scoped<S: Into<String>>(&self, prefix: S) -> Self {
        SubscribeFilter {
            path: format!("{}/{}", prefix.into(), &self.path),
            qos: self.qos,
        }
    }
}

impl From<Payload> for Bytes {
    fn from(payload: Payload) -> Self {
        payload.bytes
    }
}

impl std::ops::Deref for Payload {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}
