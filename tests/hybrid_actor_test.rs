extern crate async_trait;
extern crate simple_actor;

use async_trait::async_trait;

use simple_actor::actor::server::actor::builder::ActorBuilder;
use simple_actor::common::{HybridHandler, MessageHandler, RequestHandler, Res};

use crate::common::Number;

mod common;

/// Type of messages
enum TestMessage {
    Inc(Number),
}

/// Type of requests
enum TestRequest {
    Get,
}

/// Type of requests' responses
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

impl HybridHandler for HybridActor {}


#[test]
fn hybrid_actor() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // Hybrid actor custom logic
        let instance = HybridActor { counter: 0 };

        // Actor which wraps custom logic
        let actor = ActorBuilder::new().build_hybrid_actor(Box::new(instance));

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