extern crate async_trait;
extern crate simple_actor;
extern crate simple_logger;

use async_trait::async_trait;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::time::Duration;

use simple_actor::{HybridHandler, MessageHandler, RequestHandler, Res};
use simple_actor::ActorBuilder;
use simple_actor::MessageScheduler;
use simple_actor::Scheduling;

use crate::common::Number;

mod common;

/// Type of messages
#[derive(Clone, Debug)]
enum TestMessage {
    Inc(Number),
}

/// Type of requests
#[derive(Debug)]
enum TestRequest {
    Get,
}

/// Type of requests' responses
#[derive(Debug)]
enum TestResponse {
    CurrentValue(Number),
}

/// Hybrid actor
struct HybridActor {
    counter: Number
}

#[async_trait]
impl MessageHandler for HybridActor {
    type Message = TestMessage;

    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        match message {
            TestMessage::Inc(incrementum) => {
                self.counter += incrementum;
                Ok(())
            }
        }
    }
}

#[async_trait]
impl RequestHandler for HybridActor {
    type Request = TestRequest;
    type Reply = TestResponse;

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            TestRequest::Get => {
                Ok(TestResponse::CurrentValue(self.counter))
            }
        }
    }
}

impl HybridHandler for HybridActor {
    fn request_handler_ref(&self) -> &dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
        self
    }

    fn request_handler_mut(&mut self) -> &mut dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
        self
    }
}


#[test]
#[allow(unused_must_use)]
fn hybrid_actor() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // Hybrid actor custom logic
        let instance = HybridActor { counter: 0 };

        // Actor which wraps custom logic
        let actor = ActorBuilder::new()
            .name("HybridActor")
            .one_shot()
            .hybrid_actor(Box::new(instance))
            .build();

        // Client for actor
        let client = actor.client();

        // Expected result
        let mut sum: u128 = 0;

        // Send 'Inc' messages
        for _ in 1..=100_000u128 {
            client.message(TestMessage::Inc(1)).await.unwrap();
            sum += 1;
        }

        // Send 'Get' request
        let TestResponse::CurrentValue(get_counter) = client.request(TestRequest::Get)
            .await
            .unwrap();
        // check counter
        assert_eq!(sum, get_counter);


        actor.stop().await.unwrap();
    })
}

#[test]
#[allow(unused_must_use)]
fn scheduled_hybrid_actor_test() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // Hybrid actor custom logic
        let instance = HybridActor { counter: 0 };

        // Actor which wraps custom logic
        let actor = ActorBuilder::new()
            .name("HybridActor")
            .one_shot()
            .hybrid_actor(Box::new(instance))
            .build();

        // Client for actor
        let client = actor.client();
        let client1 = actor.message_client();

        // start actor message scheduler
        let message_scheduler = MessageScheduler::new(TestMessage::Inc(1), Scheduling::Periodic(Duration::from_millis(30)), client1);

        // waiting for 100ms
        tokio::time::sleep(Duration::from_millis(100)).await;
        // abort scheduler
        message_scheduler.abort();

        // If message processing causes error, actor stops.
        // The error is available by return value of stop method.
        // Send invalid value: 0
        // The message sending was succeeded, so it returns with Ok()
        let TestResponse::CurrentValue(get_counter) = client.request(TestRequest::Get)
            .await
            .unwrap();
        // check counter
        assert!(get_counter >= 3);

        // Stop the actor
        actor.stop().await.unwrap();
    })
}