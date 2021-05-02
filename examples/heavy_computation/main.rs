extern crate async_trait;
extern crate chrono;

use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;

use simple_actor::actor::server::actor::builder::ActorBuilder;
use simple_actor::actor::server::actor::request::RequestActor;
use simple_actor::common::{RequestHandler, Res};

/// Computation requests
#[derive(Debug)]
enum ComputationRequests {
    /// Synchronous request with simple and sort computation
    SoftComputation(u32),
    /// Asynchronous request which simulates long and heavy computation
    HeavyComputation(u32),
    /// Result of `HeavyComputation` which is processed as `HeavyComputationInternalResult`
    /// synchronous request
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

/// Actor which can perform soft and heavy computation
struct ComputationActor {
    /// Maximal message buffer size
    buffer_size: usize,
    /// Stores result of all heavy computation to demonstrate how can be modify the actor's state
    /// from asynchronous request
    heavy_computation_results: Vec<u32>,
}

impl ComputationActor {
    /// Creates new instance
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            heavy_computation_results: Vec::with_capacity(128),
        }
    }

    /// Starts new actor
    pub fn start(self) -> RequestActor<ComputationRequests, ComputationResults> {
        ActorBuilder::new()
            .name("computation")
            .receive_buffer_size(self.buffer_size)
            .build_request_actor(Box::new(self))
    }

    /// Perform heavy computations
    pub fn heavy_computation(request: ComputationRequests) -> Res<ComputationRequests> {
        if let ComputationRequests::HeavyComputation(arg) = request {
            println!("{}: Heavy computation has started", Utc::now());
            // Simulate heavy computation with sleep
            std::thread::sleep(Duration::from_millis(arg as u64));
            println!("{}: Heavy computation has stopped", Utc::now());
            // Heavy computation is ready. Convert its result to 'normal' synchronous request,
            // which will be sent to the actor again. This is such a partial result.
            // It can be processed in the `RequestHandler` implementation, at the
            // `process_request` function. This function could change the actor's state.
            Ok(ComputationRequests::HeavyComputationInternalResult(arg * 100))
        } else {
            Err(format!("{:?} asynchronous request is not handled", request).into())
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
                println!("{}: Soft computation has calculated with result: {}", Utc::now(), res);
                Ok(ComputationResults::SoftResult(res))
            }
            // The result of heavy computation will be processed by synchronous request
            ComputationRequests::HeavyComputationInternalResult(res) => {
                println!("{}: Heavy computation has calculated with result: {}", Utc::now(), res);
                self.heavy_computation_results.push(res);
                // Reply to requester client
                Ok(ComputationResults::HeavyResult(res))
            }
            _ =>
                Err(format!("{:?} synchronous request has not handled", request).into())
        }
    }

    /// Checks is the request a heavy computation which has to execute by separate thread
    fn is_heavy(&self, request: &Self::Request) -> bool {
        match request {
            ComputationRequests::HeavyComputation(_) => true,
            _ => false
        }
    }

    /// Return the function which will get the heavy computation request, execute it and
    /// transform its result into an another request
    fn get_heavy_transformation(&self) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        Box::new(ComputationActor::heavy_computation)
    }
}

#[tokio::main]
pub async fn main() {
    // Create computation logic
    let computation = ComputationActor::new(128);
    // Start actor
    let computation_actor = computation.start();
    // Create client for current thread
    let computation_client = computation_actor.client();
    // Create client for spanned thread
    let computation_client2 = computation_actor.client();

    // start heavy computation first, which keeps until 100 ms
    let promise = tokio::spawn(async move {
        computation_client2.request(ComputationRequests::HeavyComputation(100)).await
    });

    // during the heavy computation runs on separate thread, the soft computation can be performed
    for i in 1..10 {
        if let ComputationResults::SoftResult(r) = computation_client.request(ComputationRequests::SoftComputation(i))
            .await.unwrap() {
            assert_eq!(i * 10, r)
        } else {
            panic!()
        }
    }

    // Get heavy computation result
    if let ComputationResults::HeavyResult(r) = promise.await.unwrap().unwrap() {
        assert_eq!(100 * 100, r);
    } else {
        panic!()
    }

    // Stop the actor
    computation_actor.stop().await.unwrap();
}