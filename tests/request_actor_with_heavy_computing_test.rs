extern crate async_trait;
extern crate simple_actor;
extern crate simple_logger;

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;
use tokio::time::Duration;

use simple_actor::{RequestHandler, Res};
use simple_actor::ActorBuilder;
use simple_actor::common::RequestExecution;

use crate::common::Number;

mod common;

#[derive(Debug)]
enum Request {
    Inc,
    Get,
    HeavyComputing,
    HeavyComputingAsync,
    HeavyComputingShorter,
    HeavyComputingReady(Number),
    HeavyComputingWithError,
    HeavyComputingCrashed,
}

#[derive(Debug)]
enum Response {
    Success,
    GetResult(Number),
    HeavyResult(Number),
}

struct TestActor {
    counter: Number,
}

impl TestActor {
    fn new() -> Self {
        TestActor { counter: 0 }
    }

    fn heavy_computing(request: Request) -> Res<Request> {
        match request {
            Request::HeavyComputing => {
                info!("Start heavy computing");
                std::thread::sleep(Duration::from_secs(2));
                info!("Stop heavy computing");
                // Generate result of the heavy computing by internal 'Request'
                // which can be processed later by simple request
                // This procedure enable to reach the TestActor state for this result processing
                // in 'process_request' function
                Ok(Request::HeavyComputingReady(100000))
            }
            Request::HeavyComputingShorter => {
                info!("Start shorter heavy computing");
                std::thread::sleep(Duration::from_millis(50));
                info!("Stop shorter heavy computing");
                // Generate result of the heavy computing by internal 'Request'
                // which can be processed later by simple request
                // This procedure enable to reach the TestActor state for this result processing
                // in 'process_request' function
                Ok(Request::HeavyComputingReady(200000))
            }
            Request::HeavyComputingWithError => {
                std::thread::sleep(Duration::from_millis(50));
                Err("error in heavy".into())
            }
            Request::HeavyComputingCrashed => {
                std::thread::sleep(Duration::from_millis(500));
                panic!()
            }
            _ => Err(format!("Invalid request:{:?}", request).into())
        }
    }

    fn heavy_computing_async(request: Request) -> Pin<Box<dyn Future<Output=Res<Request>> + Send>> {
        Box::pin(async move {
            match request {
                Request::HeavyComputingAsync => {
                    info!("Start asynchronous heavy computing");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    info!("Stop asynchronous heavy computing");
                    // Generate result of the heavy computing by internal 'Request'
                    // which can be processed later by simple request
                    // This procedure enable to reach the TestActor state for this result processing
                    // in 'process_request' function
                    Ok(Request::HeavyComputingReady(200000))
                }
                _ => Err(format!("Invalid request:{:?}", request).into())
            }
        })
    }
}

#[async_trait]
impl RequestHandler for TestActor {
    type Request = Request;
    type Reply = Response;

    async fn classify_request(&mut self, request: Self::Request) -> RequestExecution<Self::Request> {
        match request {
            // Is the request required heavy computing?
            a @ Request::HeavyComputing => RequestExecution::Blocking(a),
            a @ Request::HeavyComputingAsync => RequestExecution::Async(a),
            a @ Request::HeavyComputingShorter => RequestExecution::Blocking(a),
            a @ Request::HeavyComputingWithError => RequestExecution::Blocking(a),
            a @ Request::HeavyComputingCrashed => RequestExecution::Blocking(a),
            s @ _ => RequestExecution::Sync(s)
        }
    }

    /// Returns handle function which generates reply from request in heavy computing process
    fn get_blocking_transformation(&self) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        Box::new(TestActor::heavy_computing)
    }

    fn get_async_transformation(&self) -> Box<dyn Fn(Self::Request) -> Pin<Box<dyn Future<Output=Res<Self::Request>> + Send>> + Send + Sync> {
        Box::new(TestActor::heavy_computing_async)
    }

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            // Simple synchronous request
            Request::Inc => {
                info!("Receive Inc request");
                self.counter += 1;
                Ok(Response::Success)
            }
            // Simple synchronous request
            Request::Get => {
                info!("Receive Get request");
                Ok(Response::GetResult(self.counter))
            }
            // The result of the heavy computing. Wrap info client's response.
            Request::HeavyComputingReady(result) => Ok(Response::HeavyResult(result)),
            _ => panic!("Illegal request")
        }
    }

    fn reply_error(&self, result: Res<Self::Reply>) {
        info!("Unable to send reply: {:?}", result)
    }
}


#[test]
#[allow(unused_must_use)]
fn request_actor() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

        let client = actor.client();
        let client_heavy = actor.client();
        let client_heavy2 = actor.client();
        let client_heavy3 = actor.client();
        let client_heavy4 = actor.client();
        let client_heavy_async = actor.client();

        let mut sum: u128 = 0;

        // starts heavy computing
        let heavy_req = tokio::spawn(async move {
            info!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputing).await
        });

        // starts async heavy computing
        let heavy_req_async = tokio::spawn(async move {
            info!("Start to send asynchronous heavy computing request");
            client_heavy_async.request(Request::HeavyComputingAsync).await
        });

        // starts heavy computing what will cause an error
        let heavy_req_err = tokio::spawn(async move {
            info!("Start to send heavy computing request with returning error");
            client_heavy2.request(Request::HeavyComputingWithError).await
        });

        // starts heavy computing but the client will not wait for the reply
        let heavy_req_abort = tokio::spawn(async move {
            info!("Start to send heavy computing and client will be aborted");
            client_heavy3.request(Request::HeavyComputingShorter).await
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        heavy_req_abort.abort();

        // starts heavy computing what will cause error, but the client will not wait for the reply
        let heavy_req_err_abort = tokio::spawn(async move {
            info!("Start to send bad heavy computing and client will be aborted");
            client_heavy4.request(Request::HeavyComputingWithError).await
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        heavy_req_err_abort.abort();

        info!("Start to send client requests...");

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

        // Result of blocking heavy computing
        if let Ok(Response::HeavyResult(res)) = heavy_req.await.unwrap() {
            info!("Got heavy result: {}", res)
        } else {
            panic!("No heavy result")
        }

        // Result of asynchronous heavy computing
        if let Ok(Response::HeavyResult(res)) = heavy_req_async.await.unwrap() {
            info!("Got async heavy result: {}", res)
        } else {
            panic!("No async heavy result")
        }

        if let Err(x) = heavy_req_err.await.unwrap() {
            info!("Got heavy error result: {}", x)
        } else {
            panic!("No heavy result")
        }

        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);
    })
}

#[test]
#[allow(unused_must_use)]
fn request_actor_blocking_actor_stopped() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

        let client_heavy = actor.client();

        // starts heavy computing
        let heavy_req = tokio::spawn(async move {
            info!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputing).await
        });

        // to ensure, heavy computing has started
        tokio::time::sleep(Duration::from_millis(100)).await;

        // stop actor during heavy computation
        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);

        // Result of blocking heavy computing
        if let Err(e) = heavy_req.await.unwrap() {
            info!("Got heavy result error: {:?}", e);
        } else {
            panic!()
        }
    })
}

#[test]
#[allow(unused_must_use)]
fn request_actor_blocking_crashed() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

        let client_heavy = actor.client();

        // starts heavy computing
        let heavy_req = tokio::spawn(async move {
            info!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputingCrashed).await
        });

        // to ensure, heavy computing has started
        tokio::time::sleep(Duration::from_millis(100)).await;


        // Result of blocking heavy computing
        if let Err(e) = heavy_req.await.unwrap() {
            info!("Got heavy result error: {:?}", e);
        } else {
            panic!()
        }

        // stop actor during heavy computation
        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);
    })
}

#[test]
#[allow(unused_must_use)]
fn request_actor_blocking_error_actor_stopped() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

        let client_heavy = actor.client();

        // starts heavy computing
        let heavy_req = tokio::spawn(async move {
            info!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputingWithError).await
        });

        // to ensure, heavy computing has started
        tokio::time::sleep(Duration::from_millis(20)).await;

        // stop actor during heavy computation
        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);

        // Result of blocking heavy computing
        if let Err(e) = heavy_req.await.unwrap() {
            info!("Got heavy result error: {:?}", e);
        } else {
            panic!()
        }
    })
}

#[test]
#[allow(unused_must_use)]
fn request_actor_async_actor_stopped() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let instance = TestActor::new();

        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .request_actor(Box::new(instance))
            .build();

        let client_heavy = actor.client();

        // starts heavy computing
        let heavy_req = tokio::spawn(async move {
            info!("Start to send heavy computing request");
            client_heavy.request(Request::HeavyComputingAsync).await
        });

        // to ensure, heavy computing has started
        tokio::time::sleep(Duration::from_millis(100)).await;

        // stop actor during heavy computation
        let exit = actor.stop().await;
        info!("Exit with: {:?}", exit);

        // Result of blocking heavy computing
        if let Err(e) = heavy_req.await.unwrap() {
            info!("Got heavy result error: {:?}", e);
        } else {
            panic!()
        }
    })
}
