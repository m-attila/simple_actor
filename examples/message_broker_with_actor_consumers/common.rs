extern crate async_trait;
extern crate simple_actor;
extern crate uuid;

use std::fmt::{Debug, Error, Formatter};

use self::uuid::Uuid;
use self::simple_actor::actor::client::message::ActorMessageClient;

/// JSON content type in messages
pub type Json = String;
/// Topic identifier
pub type Topic = String;
/// Consumer actor's client reference for subscribing
pub type ConsumerRef = Box<dyn ActorMessageClient<Message=ConsumerMessages>>;
/// Unique if of consumer client for subscribing and unsubscribing
pub type ConsumerId = Uuid;

/// Message type for message broker
#[derive(Debug, Clone)]
pub struct JsonMessage {
    /// Topic ID
    topic: Topic,
    /// JSON content
    body: Json,
}

/// JsonMessage's traits for async operations
unsafe impl Send for JsonMessage {}

unsafe impl Sync for JsonMessage {}

impl JsonMessage {
    /// Create new message
    pub fn new(topic: Topic, body: Json) -> Self {
        Self { topic, body }
    }

    /// Returns topic ID
    pub fn topic(&self) -> &Topic {
        &self.topic
    }

    /// Returns JSON body
    pub fn body(&self) -> &Json { &self.body }
}

/// Message to MessageBroker
#[derive(Debug)]
pub enum BrokerMessages {
    /// Push new message intro broker queue
    Push(JsonMessage),
}

/// Requests to MessageBroker
pub enum BrokerRequests {
    /// New consumer client subscription for a topic
    Subscribe(Topic, ConsumerId, ConsumerRef),
    /// Consumer client unsubscription from the topic
    Unsubscribe(Topic, ConsumerId),
}

/// Possible responses for MessageBroker's requests
#[derive(Debug, PartialEq)]
pub enum BrokerResponses {
    /// Subscription was succeeded
    Subscribed,
    /// Unsubscription was succeeded
    Unsubscribed,
}

/// Debug implementation for BrokerRequest
/// Manual implementation prevents that the ConsumerRef needs to be implement Debug trait
impl Debug for BrokerRequests {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            BrokerRequests::Subscribe(topic, id, _) => {
                f.write_fmt(format_args!("Subscribe: {} -> {}", topic, id))
            }
            BrokerRequests::Unsubscribe(topic, id) => {
                f.write_fmt(format_args!("Unsubscribe: {} -> {}", topic, id))
            }
        }
    }
}

/// Message to consumer actors
#[derive(Debug)]
pub enum ConsumerMessages {
    /// Message broker publish a new message towards the consumer
    Publish(Topic, JsonMessage),
}