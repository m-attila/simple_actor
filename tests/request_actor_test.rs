extern crate async_trait;
extern crate simple_actor;

use async_trait::async_trait;

use simple_actor::actor::server::actor::builder::ActorBuilder;
use simple_actor::common::{ActorError, RequestHandler, Res};

mod common;
use crate::common::Number;

enum Request {
    Inc(Number),
    Get,
}

enum Response {
    Success,
    CurrentValue(Number),
}

/// test actor
struct TestActor {
    counter: Number
}

#[async_trait]
impl RequestHandler for TestActor {
    type Request = Request;
    type Reply = Response;

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            Request::Inc(incrementum) => {
                if self.counter >= 50_000 {
                    Err(ActorError::from("Upper limit has exceeded."))
                } else {
                    self.counter += incrementum;
                    Ok(Response::Success)
                }
            }
            Request::Get => {
                Ok(Response::CurrentValue(self.counter))
            }
        }
    }
}

#[test]
fn request_actor() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor { counter: 0 };

        let actor = ActorBuilder::new().build_request_actor(Box::new(instance));
        let client = actor.client();

        let mut sum: u128 = 0;

        for _ in 1..=100_000u128 {
            if let Err(_) = client.request(Request::Inc(1)).await {
                break;
            } else {
                sum += 1;
            };
        }

        if let Response::CurrentValue(get_counter) = client.request(Request::Get).await.unwrap() {
            assert_eq!(sum, get_counter);
        } else { panic!("Bad response!") }

        let exit = actor.stop().await;
        println!("Exit with: {:?}", exit);
    })
}

