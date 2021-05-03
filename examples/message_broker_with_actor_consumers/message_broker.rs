extern crate async_trait;
extern crate uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use futures::future::join_all;

use simple_actor::ActorBuilder;
use simple_actor::actor::server::actor::hybrid::HybridActor;
use simple_actor::common::{HybridHandler, MessageHandler, RequestHandler, Res};

use crate::common::{BrokerMessages, BrokerRequests, BrokerResponses, ConsumerId, ConsumerMessages, ConsumerRef, JsonMessage, Topic};

struct MessageBroker(HashMap<Topic, HashMap<ConsumerId, ConsumerRef>>);

/// Default implementation for MessageBroker
impl Default for MessageBroker {
    /// Creates default instance
    fn default() -> Self {
        Self(HashMap::with_capacity(16))
    }
}

impl MessageBroker {
    /// Subscribe new consumer for the topic
    fn add(&mut self, topic: Topic, id: ConsumerId, client: ConsumerRef) -> Result<(), String> {
        let entry = self.0
            .entry(topic)
            .or_insert(HashMap::with_capacity(16));

        match entry.contains_key(&id) {
            true => Err(format!("{} client already subscribed for the topic", id)),
            false => {
                entry.insert(id, client);
                Ok(())
            }
        }
    }

    /// Unsubscribe consumer from the topic
    fn rem(&mut self, topic: Topic, id: &ConsumerId) -> Result<(), String> {
        match self.0.get_mut(&topic) {
            None => Err(format!("{} topic does not exist", &topic)),
            Some(t) => {
                t.remove(id)
                    .map_or_else(|| Err(format!("{} client has not subscribed for the topic yet", id)),
                                 |_| Ok(()))
            }
        }
    }

    /// Broke the message into several topics
    async fn push(&self, message: JsonMessage) {
        // Gets subscribed consumer clients
        let clients = self.0
            .get(message.topic())
            .map(|ts| ts.iter());

        if let Some(clients) = clients {
            let futures: Vec<_> = clients
                .map(|(_, client)| (client, message.clone()))
                .map(|(client, message)| client.message(ConsumerMessages::Publish(message.topic().clone(),message)))
                .collect();
            let errors = join_all(futures).await
                .iter_mut()
                .filter(|r| r.is_err())
                .count();
            if errors > 0 {
                panic!("There was some errors")
            }
        }
    }
}

/// Wraps MessageBroker into HybridActor which can accept messages and requests
pub struct MessageBrokerActor {
    /// The broker
    broker: MessageBroker,
    /// Maximal message buffer size
    buffer_size: usize,
}

impl MessageBrokerActor {
    /// Create new instance with given message buffer size
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            broker: MessageBroker::default(),
        }
    }

    /// Starts new actor
    pub fn start(self) -> HybridActor<BrokerMessages, BrokerRequests, BrokerResponses> {
        ActorBuilder::new()
            .name("message_broker")
            .receive_buffer_size(self.buffer_size)
            .one_shot()
            .hybrid_actor(Box::new(self))
            .build()
    }
}

/// MessageBrokerActor accepts messages and requests
impl HybridHandler for MessageBrokerActor {}

/// How to handle Messages
#[async_trait]
impl MessageHandler for MessageBrokerActor {
    type Message = BrokerMessages;

    /// Handle Push message which will be deal among the subscribed consumer clients
    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        let BrokerMessages::Push(json) = message;
        self.broker.push(json).await;
        Ok(())
    }
}

/// How to handle requests
#[async_trait]
impl RequestHandler for MessageBrokerActor {
    type Request = BrokerRequests;
    type Reply = BrokerResponses;

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        let result = match request {
            // Subscribe request was received
            BrokerRequests::Subscribe(topic, id, client) =>
                self.broker.add(topic, id, client).map(|_| BrokerResponses::Subscribed),
            // Unsubscribe request was received
            BrokerRequests::Unsubscribe(topic, id) =>
                self.broker.rem(topic, &id).map(|_| BrokerResponses::Unsubscribed)
        };
        // maps String error into ActorError
        result.map_err(|s| s.into())
    }
}