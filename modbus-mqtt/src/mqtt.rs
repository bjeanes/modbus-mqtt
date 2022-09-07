use std::collections::HashMap;

use bytes::Bytes;
use rumqttc::{
    mqttbytes::matches as matches_topic, mqttbytes::valid_topic, AsyncClient, Event, EventLoop,
    MqttOptions, Publish, Subscribe, SubscribeFilter,
};
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
};
use tracing::{debug, warn};

use crate::shutdown::Shutdown;

#[derive(Debug)]
pub enum Message {
    Subscribe(Subscribe, Sender<Bytes>),
    Publish(Publish),
    Shutdown,
}

pub(crate) async fn new(options: MqttOptions, shutdown: Shutdown) -> Connection {
    let (client, event_loop) = AsyncClient::new(options, 32);

    let (tx, rx) = channel(32);
    Connection {
        client,
        event_loop,
        subscriptions: HashMap::new(),
        tx,
        rx,
        shutdown,
    }
}

// Maintain internal subscriptions as well as MQTT subscriptions. Relay all received messages on MQTT subscribed topics
// to internal components who have a matching topic. Unsubscribe topics when no one is listening anymore.
pub(crate) struct Connection {
    subscriptions: HashMap<String, Vec<Sender<Bytes>>>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    client: AsyncClient,
    event_loop: EventLoop,
    shutdown: Shutdown,
}

impl Connection {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            select! {
                event = self.event_loop.poll() => {
                    match event {
                        Ok(event) => self.handle_event(event).await?,
                        _ => todo!()
                    }
                }
                request = self.rx.recv() => {
                    match request {
                        None => return Ok(()),
                        Some(Message::Shutdown) => return Ok(()),
                        Some(req) => self.handle_request(req).await?,
                    }
                }
                _ = self.shutdown.recv() => return Ok(())
            }
        }
    }

    /// Create a handle for interacting with the MQTT server such that a pre-provided prefix is transparently added to
    /// all relevant commands which use a topic.
    pub fn prefixed_handle<S: Into<String> + Send>(
        &self,
        prefix: S,
    ) -> crate::Result<Sender<Message>> {
        let prefix = prefix.into();

        if !valid_topic(&prefix) {
            return Err("Prefix is not a valid topic".into());
        }

        let inner_tx = self.handle();
        let (wrapper_tx, mut wrapper_rx) = channel::<Message>(8);

        let prefix: String = prefix.into();

        tokio::spawn(async move {
            while let Some(msg) = wrapper_rx.recv().await {
                if inner_tx.send(msg.prefixed(prefix.clone())).await.is_err() {
                    break;
                }
            }
        });

        Ok(wrapper_tx)
    }

    pub fn handle(&self) -> Sender<Message> {
        self.tx.clone()
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
    async fn handle_data(&mut self, topic: String, payload: Bytes) -> crate::Result<()> {
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
            if target.send(payload.clone()).await.is_err() {
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

trait Prefixable {
    fn prefixed<S: Into<String>>(self, prefix: S) -> Self;
}

impl Prefixable for Message {
    fn prefixed<S: Into<String>>(self, prefix: S) -> Self {
        match self {
            Message::Subscribe(sub, bytes) => Message::Subscribe(sub.prefixed(prefix), bytes),
            Message::Publish(publish) => Message::Publish(publish.prefixed(prefix)),
            other => other,
        }
    }
}

impl Prefixable for Subscribe {
    fn prefixed<S: Into<String>>(mut self, prefix: S) -> Self {
        let prefix: String = prefix.into();
        Self {
            pkid: self.pkid,
            filters: self
                .filters
                .iter_mut()
                .map(|f| f.clone().prefixed(prefix.clone()))
                .collect(),
        }
    }
}

impl Prefixable for Publish {
    fn prefixed<S: Into<String>>(self, prefix: S) -> Self {
        let mut prefixed = self.clone();
        prefixed.topic = format!("{}/{}", prefix.into(), &self.topic);
        prefixed
    }
}

impl Prefixable for SubscribeFilter {
    fn prefixed<S: Into<String>>(self, prefix: S) -> Self {
        SubscribeFilter {
            path: format!("{}/{}", prefix.into(), &self.path),
            qos: self.qos,
        }
    }
}
