//! This example introduces how can be handle a long heavy computation without blocking actor's processing loop.
extern crate async_trait;
extern crate chrono;

use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;

use simple_actor::actor::server::actor::request::RequestActor;
use simple_actor::common::{RequestExecution, RequestHandler, Res};
use simple_actor::ActorBuilder;

/// Computation requests
#[derive(Debug)]
enum ComputationRequests {
    /// Synchronous request with a simple and short computation
    SoftComputation(u32),
    /// Asynchronous request which simulates a long and heavy computation
    HeavyComputation(u32),
    /// The result of `HeavyComputation` generates `HeavyComputationInternalResult`
    /// which is a synchronous request to actor itself.
    HeavyComputationInternalResult(u32),
}

/// Computation results
#[derive(Debug)]
enum ComputationResults {
    /// Result of `SoftComputation` request
    SoftResult(u32),
    /// Result of `HeavyComputation` request
    HeavyResult(u32),
}

/// Actor which can perform soft and heavy computation as well
struct ComputationActor {
    ///  Message buffer size
    buffer_size: usize,
    /// Store results of all heavy computation to demonstrate how can be modify the actor's internal state
    /// from an asynchronous request.
    heavy_computation_results: Vec<u32>,
}

impl ComputationActor {
    /// Create new instance
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            heavy_computation_results: Vec::with_capacity(128),
        }
    }

    /// Start a new actor
    pub fn start(self) -> RequestActor<ComputationRequests, ComputationResults> {
        ActorBuilder::new()
            .name("computation")
            .receive_buffer_size(self.buffer_size)
            .one_shot()
            .request_actor(Box::new(self))
            .build()
    }

    /// Perform heavy computation
    pub fn heavy_computation(request: ComputationRequests) -> Res<ComputationRequests> {
        if let ComputationRequests::HeavyComputation(arg) = request {
            println!("{}: Heavy computation has been started...", Utc::now());
            // To simulate the heavy computation with a sleeping function
            std::thread::sleep(Duration::from_millis(arg as u64));
            println!("{}: Heavy computation has stopped", Utc::now());
            // Heavy computation is ready. Convert its result to 'normal' synchronous request,
            // which will be sent to the actor again. This is such a partial result,
            // which can be processed in the `process_request` function of `RequestHandler` implementation.
            // This function could change the actor's state.
            Ok(ComputationRequests::HeavyComputationInternalResult(
                arg * 100,
            ))
        } else {
            Err(format!("{:?} asynchronous request has not handled", request).into())
        }
    }
}

#[async_trait]
impl RequestHandler for ComputationActor {
    type Request = ComputationRequests;
    type Reply = ComputationResults;

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            // Simple synchronous request
            ComputationRequests::SoftComputation(arg) => {
                let res = arg * 10;
                println!(
                    "{}: Soft computation has calculated this result: {}",
                    Utc::now(),
                    res
                );
                Ok(ComputationResults::SoftResult(res))
            }
            // The result of heavy computation will be processed by a synchronous request
            ComputationRequests::HeavyComputationInternalResult(res) => {
                println!(
                    "{}: Heavy computation has calculated this result: {}",
                    Utc::now(),
                    res
                );
                // change actor's internal state by storing the result
                self.heavy_computation_results.push(res);
                // then reply to the client...
                Ok(ComputationResults::HeavyResult(res))
            }
            _ => Err(format!("{:?} synchronous request has not handled", request).into()),
        }
    }

    /// Check the request is a heavy computation which has to execute in a separate task
    async fn classify_request(
        &mut self,
        request: Self::Request,
    ) -> RequestExecution<Self::Request> {
        match request {
            a @ ComputationRequests::HeavyComputation(_) => RequestExecution::Blocking(a),
            s @ _ => RequestExecution::Sync(s),
        }
    }

    /// Return the callback function which will get a heavy computation request to process,
    /// and transforms its result into an another new request
    fn get_blocking_transformation(
        &self,
    ) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        Box::new(ComputationActor::heavy_computation)
    }
}

#[tokio::main]
pub async fn main() {
    main_internal().await
}

async fn main_internal() {
    // Create computation logic
    let computation = ComputationActor::new(128);
    // Start actor
    let computation_actor = computation.start();
    // Create client to use in current thread
    let computation_client = computation_actor.client();
    // Create client to use in a spanned task
    let computation_client2 = computation_actor.client();

    // start a heavy computation first, which keeps until 100 ms
    let promise = tokio::spawn(async move {
        computation_client2
            .request(ComputationRequests::HeavyComputation(100))
            .await
    });

    // during the previous heavy computation which runs on separate task, this soft computation can be performed
    for i in 1..10 {
        if let ComputationResults::SoftResult(r) = computation_client
            .request(ComputationRequests::SoftComputation(i))
            .await
            .unwrap()
        {
            assert_eq!(i * 10, r)
        } else {
            panic!()
        }
    }

    // Get heavy computation's result
    if let ComputationResults::HeavyResult(r) = promise.await.unwrap().unwrap() {
        assert_eq!(100 * 100, r);
    } else {
        panic!()
    }

    // Stop the actor
    computation_actor.stop().await.unwrap();
}

#[tokio::test]
async fn my_test() {
    main_internal().await
}
