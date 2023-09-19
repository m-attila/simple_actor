//! This example introduces how can be changed consumer trait objects to actors.
//! Actors work as similarly than consumers in `message_broker_with_dyn_consumers` example.
//!
//! **The message broker**
//! is a hybrid actor, which can handle asynchronous messages and synchronous requests as well.
//! This actor receives `subscribe` and `unsubscribe` requests
//! for `Info`, `Warning` and `Error` topics, and receives events as `system` messages. These can be `info`, `warning` or `error`.
//! A message consist from a JSON description which contains all features of an event.
//!
//! **Consumer actors**
//! can subscribe for any topic, whereto the message broker sends messages.
//! The actor makes a subscription by using its unique ID and client interface.
//!
//! In this example there are two consumer.
//!
//! First one, is the `DevOpsEventConsumerActor` which subscribes for warning and error topics, and give an alert
//! when such event has received.
//!
//! The other one, is the `LogConsumerActor` which consumes the messages
//! from all of topics, simulates a log writer mechanism on the console.
//! This actor has an extra function, which demonstrates, all actor could be an own state, which could be modified during
//! the message processing. `LogConsumerActor` has a topic level counter, which stores the number of received messages for each topics.
//!
//! The state-handling mechanism does not require any locking because the message processing is serialized.

extern crate ansi_term;
extern crate async_trait;
extern crate rand;
extern crate serde_json;
extern crate uuid;

use std::collections::HashMap;

use ansi_term::Colour;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use simple_actor::common::{MessageHandler, Res};
use simple_actor::ActorBuilder;

use crate::common::{
    BrokerMessages, BrokerRequests, BrokerResponses, ConsumerMessages, JsonMessage,
};
use crate::message_broker::MessageBrokerActor;
use crate::topic::Topic;

mod common;
mod message_broker;
mod topic;

/// Consumer actor which accepts error and warning messages only
struct DevOpsEventConsumerActor {
    /// Unique identifier of the consumer
    uuid: Uuid,
}

impl DevOpsEventConsumerActor {
    fn uuid(&self) -> Uuid {
        self.uuid.clone()
    }
}

impl Default for DevOpsEventConsumerActor {
    /// Creates default instance
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }
}

#[async_trait]
impl MessageHandler for DevOpsEventConsumerActor {
    type Message = ConsumerMessages;

    /// Implement `MessageHandler` trait to handle critical messages (error, warnings) and gives an alert
    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        let ConsumerMessages::Publish(_, json) = message;
        // 'Alerting' function
        println!(
            "{} message: {}",
            Colour::Red.bold().paint("Alert"),
            Colour::Yellow.paint(format!("{:?}", json.body()))
        );
        Ok(())
    }
}

/// Consumer which accepts all type of messages like a logger
struct LoggerConsumerActor {
    /// Unique identifier of the consumer
    uuid: Uuid,
    // Counter for each topic
    topic_stat: HashMap<common::Topic, u32>,
}

impl LoggerConsumerActor {
    fn uuid(&self) -> Uuid {
        self.uuid.clone()
    }
}

impl Default for LoggerConsumerActor {
    /// Create default instance
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            topic_stat: HashMap::with_capacity(3),
        }
    }
}

#[async_trait]
impl MessageHandler for LoggerConsumerActor {
    type Message = ConsumerMessages;

    /// Implement `MessageHandler` trait to handle all messages (info, error, warnings) and logs out them into the console
    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        let ConsumerMessages::Publish(topic, json) = message;
        // Modify topic counter

        let c_topic = topic.clone();

        let topic_cntr = self
            .topic_stat
            .entry(topic)
            .and_modify(|prev| *prev += 1)
            .or_insert(1);

        // 'Logging' function
        println!(
            "{}-{}: {}: {:?}",
            c_topic,
            topic_cntr,
            Colour::Cyan.bold().paint("logfile"),
            json.body()
        );
        Ok(())
    }
}

#[tokio::main]
pub async fn main() {
    main_internal().await
}
async fn main_internal() {
    // Create actor
    let broker = MessageBrokerActor::new(128);
    // Start it
    let broker_actor = broker.start();
    // Get client to send messages
    let broker_client = broker_actor.client();

    // Create devops consumer actor to handle critical events
    let devops_logic = DevOpsEventConsumerActor::default();
    // Get unique ID for subscription
    let devops_id = devops_logic.uuid();
    let devops = ActorBuilder::new()
        .receive_buffer_size(128)
        .name("devops")
        .one_shot()
        .message_actor(Box::new(devops_logic))
        .build();

    // Subscribe for Error topic
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Error.into(),
                devops_id,
                devops.client()
            ))
            .await
            .unwrap()
    );
    // Subscribe for Warning topic
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Warning.into(),
                devops_id,
                devops.client()
            ))
            .await
            .unwrap()
    );

    // Create logger consumer which can accept all type of messages
    let logger_logic = LoggerConsumerActor::default();
    // Get unique ID for subscription
    let logger_id = logger_logic.uuid();
    let logger = ActorBuilder::new()
        .receive_buffer_size(128)
        .name("logger")
        .one_shot()
        .message_actor(Box::new(logger_logic))
        .build();

    // Subscribe for all topics
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Info.into(),
                logger_id,
                logger.client()
            ))
            .await
            .unwrap()
    );
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Warning.into(),
                logger_id,
                logger.client()
            ))
            .await
            .unwrap()
    );
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Error.into(),
                logger_id,
                logger.client()
            ))
            .await
            .unwrap()
    );

    // Send 100 random type of messages to the message broker
    for i in 1..100 {
        // Select topic randomly
        let topic: Topic = rand::random();
        let time = Utc::now();

        // Create JSON message which depends on topic type
        let data = match topic {
            Topic::Info => {
                json!({"id":i, "event": "Some info event", "time":time})
            }
            Topic::Warning => {
                json!({"id":i, "event": "Some warning event", "category":"I/O", "time":time})
            }
            Topic::Error => {
                json!({"id":i, "event": "Some error event", "operation":"PUT", "source":"local_server", "time":time})
            }
        };

        // Send data into the message broker
        broker_client
            .message(BrokerMessages::Push(JsonMessage::new(
                topic.into(),
                data.to_string(),
            )))
            .await
            .unwrap();
    }

    // Unsubscribe devops consumer's client
    assert_eq!(
        BrokerResponses::Unsubscribed,
        broker_client
            .request(BrokerRequests::Unsubscribe(Topic::Error.into(), devops_id))
            .await
            .unwrap()
    );
    assert_eq!(
        BrokerResponses::Unsubscribed,
        broker_client
            .request(BrokerRequests::Unsubscribe(
                Topic::Warning.into(),
                devops_id
            ))
            .await
            .unwrap()
    );

    // Unsubscribe logger consumer's client
    assert_eq!(
        BrokerResponses::Unsubscribed,
        broker_client
            .request(BrokerRequests::Unsubscribe(Topic::Info.into(), logger_id))
            .await
            .unwrap()
    );
    assert_eq!(
        BrokerResponses::Unsubscribed,
        broker_client
            .request(BrokerRequests::Unsubscribe(
                Topic::Warning.into(),
                logger_id
            ))
            .await
            .unwrap()
    );
    assert_eq!(
        BrokerResponses::Unsubscribed,
        broker_client
            .request(BrokerRequests::Unsubscribe(Topic::Error.into(), logger_id))
            .await
            .unwrap()
    );

    devops.stop().await.unwrap();
    logger.stop().await.unwrap();
    // Stop the message broker actor
    broker_actor.stop().await.unwrap();
}

#[tokio::test]
async fn my_test() {
    main_internal().await
}
