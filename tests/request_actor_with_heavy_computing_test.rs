extern crate async_trait;
extern crate simple_actor;


use async_trait::async_trait;
use tokio::time::Duration;

use simple_actor::actor::server::actor::builder::ActorBuilder;
use simple_actor::common::{RequestHandler, Res};

use crate::common::Number;

mod common;

#[derive(Debug)]
enum Request {
    Inc,
    Get,
    HeavyComputing,
    HeavyComputingReady(Number)
}

enum Response {
    Success,
    GetResult(Number),
    HeavyResult(Number),
}

struct TestActor {
    counter: Number
}

impl TestActor {
    fn new() -> Self {
        TestActor { counter: 0 }
    }

    fn heavy_computing(request: Request) -> Res<Request> {
        if let Request::HeavyComputing = request {
            println!("Start heavy computing");
            std::thread::sleep(Duration::from_secs(2));
            println!("Stop heavy computing");
            // Generate result of the heavy computing by internal 'Request'
            // which can be processed later by simple request
            // This procedure enable to reach the TestActor state for this result processing
            // in 'process_request' function
            Ok(Request::HeavyComputingReady(100000))
        } else {
            Err(format!("Invalid request:{:?}", request).into())
        }
    }
}

#[async_trait]
impl RequestHandler for TestActor {
    type Request = Request;
    type Reply = Response;

    fn is_heavy(&self, request: &Self::Request) -> bool {
        match request {
            // Is the request required heavy computing?
            Request::HeavyComputing => true,
            _ => false
        }
    }
    /// Returns handle function which generates reply from request in heavy computing process
    fn get_heavy_transformation(&self) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        Box::new(TestActor::heavy_computing)
    }

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            // Simple synchronous request
            Request::Inc => {
                println!("Receive Inc request");
                self.counter += 1;
                Ok(Response::Success)
            }
            // Simple synchronous request
            Request::Get => {
                println!("Receive Get request");
                Ok(Response::GetResult(self.counter))
            }
            // The result of the heavy computing. Wrap info client's response.
            Request::HeavyComputingReady(result) => Ok(Response::HeavyResult(result)),
            _ => panic!("Illegal request")
        }
    }
}


#[test]
fn request_actor() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new().build_request_actor(Box::new(instance));
        let client = actor.client();
        let client_heavy=actor.client();

        let mut sum: u128 = 0;

        // starts heavy computing
        let heavy_req =tokio::spawn(async move{
            println!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputing).await
        });

        println!("Start to send client requests...");

        for _ in 1..=100u128 {
            if let Err(_) = client.request(Request::Inc).await {
                break;
            } else {
                sum += 1;
            };
        }

        if let Response::GetResult(get_counter) = client.request(Request::Get).await.unwrap() {
            assert_eq!(sum, get_counter);
        } else { panic!("Bad response!") }

        if let Ok(Response::HeavyResult(res))= heavy_req.await.unwrap(){
            println!("Got heavy result: {}", res)
        }
        else {
            panic!("No heavy result")
        }

        let exit = actor.stop().await;
        println!("Exit with: {:?}", exit);
    })
}



