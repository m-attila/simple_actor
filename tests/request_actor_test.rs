extern crate async_trait;
extern crate simple_actor;
extern crate simple_logger;

use async_trait::async_trait;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;

use simple_actor::ActorBuilder;
use simple_actor::{ActorError, RequestHandler, Res};

use crate::common::Number;

mod common;

#[derive(Debug)]
enum Request {
    Inc(Number),
    Get,
}

#[derive(Debug)]
enum Response {
    Success,
    CurrentValue(Number),
}

/// test actor
struct TestActor {
    counter: Number,
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
            Request::Get => Ok(Response::CurrentValue(self.counter)),
        }
    }
}

#[test]
#[allow(unused_must_use)]
fn request_actor() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor { counter: 0 };

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

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
        } else {
            panic!("Bad response!")
        }

        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);

        // Try to send after actor has stopped
        client.request(Request::Inc(1)).await.unwrap_err();
    })
}
