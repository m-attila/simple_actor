//! This example introduces simple usage of an HybridActor. This actor can handle asynchronous
//! messages and synchronous requests. In this example the MessageBroker actor
//! receives subscribe and unsubscribe requests for Info/Warning/Error topics, and receives system
//! event messages which can be infos, warnings or errors. This messages contains a JSON description
//! which contains the events' features. Consumer clients could subscribe for any topic, where to
//! the message broker deals out the messages. In subscribes, client gives their unique ID and interface
//! of an trait object to the message broker. In this example there are two consumer. First one, is
//! the `DevOpsEventConsumer` which subscribes for warning and error topics, and give an alert
//! when such event has received. The other one, is the `LogConsumer` which consumes the messages
//! from all of topics, simulates a log writer mechanism to the console.

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

use crate::common::{BrokerMessages, BrokerRequests, BrokerResponses, Consumer, JsonMessage};
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
    /// Creates default instance
    fn default() -> Self {
        Self { uuid: Uuid::new_v4() }
    }
}

impl Consumer for DevOpsEventConsumer {
    /// Implements Consumer trait to handle critical messages (error, warnings) and gives an alert
    fn consume(&self, message: JsonMessage) {
        // 'Alerting' function
        println!("{} message: {}",
                 Colour::Red.bold().paint("Alert"),
                 Colour::Yellow.paint(format!("{:?}", message.body())));
    }
}

/// Consumer which accepts all type of messages like a logger
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
    /// Creates default instance
    fn default() -> Self {
        Self { uuid: Uuid::new_v4() }
    }
}

impl Consumer for LoggerConsumer {
    /// Implements Consumer trait to handle all messages (info, error, warnings) and logs out them into the console
    fn consume(&self, message: JsonMessage) {
        // 'Logging' function
        println!("{}: {:?}",
                 Colour::Cyan.bold().paint("logfile"),
                 message.body());
    }
}

#[tokio::main]
pub async fn main() {
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
    // Get Consumer trait's reference to accept messages
    let devops_ref: Arc<Mutex<dyn Consumer>> = Arc::new(Mutex::new(devops));

    // Subscribe for Error topic
    assert_eq!(BrokerResponses::Subscribed,
               broker_client.request(BrokerRequests::Subscribe(Topic::Error.into(), devops_id, Arc::clone(&devops_ref)))
                   .await
                   .unwrap());
    // Subscribe for Warning topic
    assert_eq!(BrokerResponses::Subscribed,
               broker_client.request(BrokerRequests::Subscribe(Topic::Warning.into(), devops_id, Arc::clone(&devops_ref)))
                   .await
                   .unwrap());

    // Create logger consumer which can accept all type of messages
    let logger = LoggerConsumer::default();
    // Get unique ID for subscription
    let logger_id = logger.uuid();
    // Get Consumer trait's reference to accept messages
    let logger_ref: Arc<Mutex<dyn Consumer>> = Arc::new(Mutex::new(logger));

    // Subscribe for all topics
    assert_eq!(BrokerResponses::Subscribed,
               broker_client.request(BrokerRequests::Subscribe(Topic::Info.into(), logger_id, Arc::clone(&logger_ref)))
                   .await
                   .unwrap());
    assert_eq!(BrokerResponses::Subscribed,
               broker_client.request(BrokerRequests::Subscribe(Topic::Warning.into(), logger_id, Arc::clone(&logger_ref)))
                   .await
                   .unwrap());
    assert_eq!(BrokerResponses::Subscribed,
               broker_client.request(BrokerRequests::Subscribe(Topic::Error.into(), logger_id, Arc::clone(&logger_ref)))
                   .await
                   .unwrap());

    // Send 100 random type of messages to message broker
    for i in 1..100 {
        // Select topic randomly
        let topic: Topic = rand::random();
        let time = Utc::now();

        // Create JSON message which depends on topic type
        let data =
            match topic {
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
        broker_client.message(BrokerMessages::Push(JsonMessage::new(
            topic.into(),
            data.to_string(),
        ))).await.unwrap();
    }

    // Unsubscribe devops consumer's client
    assert_eq!(BrokerResponses::Unsubscribed,
               broker_client.request(BrokerRequests::Unsubscribe(Topic::Error.into(), devops_id))
                   .await
                   .unwrap());
    assert_eq!(BrokerResponses::Unsubscribed,
               broker_client.request(BrokerRequests::Unsubscribe(Topic::Warning.into(), devops_id))
                   .await
                   .unwrap());

    // Unsubscribe logger consumer's client
    assert_eq!(BrokerResponses::Unsubscribed,
               broker_client.request(BrokerRequests::Unsubscribe(Topic::Info.into(), logger_id))
                   .await
                   .unwrap());
    assert_eq!(BrokerResponses::Unsubscribed,
               broker_client.request(BrokerRequests::Unsubscribe(Topic::Warning.into(), logger_id))
                   .await
                   .unwrap());
    assert_eq!(BrokerResponses::Unsubscribed,
               broker_client.request(BrokerRequests::Unsubscribe(Topic::Error.into(), logger_id))
                   .await
                   .unwrap());

    // Stop the message broker actor
    broker_actor.stop().await.unwrap();
}
