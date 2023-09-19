//! This example introduces a simple usage of an `HybridActor`.
//! This is similar to `message_broker_with_dyn_consumers` example, but the consumer aren't an actor,
//! it is a simple trait object.
//!
//! **The message broker**
//! is a hybrid actor, which can handle asynchronous messages and synchronous requests as well.
//! This actor receives `subscribe` and `unsubscribe` requests
//! for `Info`, `Warning` and `Error` topics, and receives events as `system` messages. These can be `info`, `warning` or `error`.
//! A message consist from a JSON description which contains all features of an event.
//!
//! **Consumer objects**
//! can subscribe for any topic, whereto the message broker sends messages.
//! The consumer makes a subscription by sending a reference to itself.
//!
//! In this example there are two consumer.
//!
//! First one, is the `DevOpsEventConsumer` which subscribes for warning and error topics, and give an alert
//! when such event has received.
//!
//! The other one, is the `LoggerConsumer` which consumes the messages
//! from all of topics, simulates a log writer mechanism on the console.

extern crate ansi_term;
extern crate rand;
extern crate serde_json;
extern crate uuid;

use std::sync::Arc;

use ansi_term::Colour;
use chrono::Utc;
use serde_json::json;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::common::{
    BrokerMessages, BrokerRequests, BrokerResponses, Consumer, ConsumerRef, JsonMessage,
};
use crate::message_broker::MessageBrokerActor;
use crate::topic::Topic;

mod common;
mod message_broker;
mod topic;

/// Consumer which accepts error and warning messages only
struct DevOpsEventConsumer {
    /// Unique identifier of the consumer
    uuid: Uuid,
}

impl DevOpsEventConsumer {
    fn uuid(&self) -> Uuid {
        self.uuid.clone()
    }
}

impl Default for DevOpsEventConsumer {
    /// Create default instance
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }
}

impl Consumer for DevOpsEventConsumer {
    /// Implement `Consumer` trait to handle critical messages (error, warnings) and to print an alert to the console.
    fn consume(&self, message: JsonMessage) {
        // 'Alerting' function
        println!(
            "{} message: {}",
            Colour::Red.bold().paint("Alert"),
            Colour::Yellow.paint(format!("{:?}", message.body()))
        );
    }
}

/// Consumer which accepts all type of messages like a file logger
struct LoggerConsumer {
    /// Unique identifier of the consumer
    uuid: Uuid,
}

impl LoggerConsumer {
    fn uuid(&self) -> Uuid {
        self.uuid.clone()
    }
}

impl Default for LoggerConsumer {
    /// Create default instance
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }
}

impl Consumer for LoggerConsumer {
    /// Implement `Consumer` trait to handle all messages (info, error, warnings) and to log out them into the console
    fn consume(&self, message: JsonMessage) {
        // 'Logging' function
        println!(
            "{}: {:?}",
            Colour::Cyan.bold().paint("logfile"),
            message.body()
        );
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

    // Create devops consumer to handle critical events
    let devops = DevOpsEventConsumer::default();
    // Get unique ID for subscription
    let devops_id = devops.uuid();
    // Get `Consumer` trait's reference to accept messages
    let devops_ref: ConsumerRef = Arc::new(Mutex::new(devops));

    // Subscribe for error topic
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Error.into(),
                devops_id,
                Arc::clone(&devops_ref),
            ))
            .await
            .unwrap()
    );
    // Subscribe for warning topic
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Warning.into(),
                devops_id,
                Arc::clone(&devops_ref),
            ))
            .await
            .unwrap()
    );

    // Create logger consumer which can accept all type of messages
    let logger = LoggerConsumer::default();
    // Get unique ID for subscription
    let logger_id = logger.uuid();
    // Get `Consumer` trait's reference to accept messages
    let logger_ref: ConsumerRef = Arc::new(Mutex::new(logger));

    // Subscribe for all topics
    assert_eq!(
        BrokerResponses::Subscribed,
        broker_client
            .request(BrokerRequests::Subscribe(
                Topic::Info.into(),
                logger_id,
                Arc::clone(&logger_ref),
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
                Arc::clone(&logger_ref),
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
                Arc::clone(&logger_ref),
            ))
            .await
            .unwrap()
    );

    // Send 100 random type of messages to message broker
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

    // Unsubscribe devops consumer client
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
                devops_id,
            ))
            .await
            .unwrap()
    );

    // Unsubscribe logger consumer client
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
                logger_id,
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

    // Stop the message broker actor
    broker_actor.stop().await.unwrap();
}

#[tokio::test]
async fn my_test() {
    main_internal().await
}
