//! This example introduces how can be sent scheduled messages periodically to an actor.
//!
//! The `SamplerActor` receives measuring data and calculate average value of them within a given
//! time window. This calculation is triggered by a periodically generated `MarkPosition` message.
//! After the calculation the actor stores the calculated value, and when a `GetAverage` is received
//! it send back all the stored calculations as a reply.
extern crate rand;

use std::time::Duration;

use rand::Rng;

use simple_actor::actor::client::scheduler::common::Scheduling;
use simple_actor::actor::client::scheduler::message::MessageScheduler;

use crate::common::{SampleMessages, SampleRequest, SampleResponse};
use crate::sampler::SamplerActor;

mod common;
mod sampler;

#[tokio::main]
pub async fn main() {
    main_internal().await
}

async fn main_internal() {
    // Create `SampleActor` instance
    let sampler = SamplerActor::new(256);
    // Start actor
    let sampler_actor = sampler.start();

    // Create client to send measuring data
    let sample_client = sampler_actor.client();
    // Create client to send periodically scheduled `MarkPosition` message
    let sample_client2 = sampler_actor.client();
    // Start message scheduler
    let msg_scheduler = MessageScheduler::new(
        SampleMessages::MarkPosition,
        Scheduling::Periodic(Duration::from_millis(10)),
        sampler_actor.message_client(),
    );

    let mut rng = rand::thread_rng();
    // Send 1000 measurement values to the actor
    for _ in 0..1000 {
        let value = rng.gen_range(0..1000);
        sample_client
            .message(SampleMessages::Store(value))
            .await
            .unwrap();
        // Wait some microseconds between the message sending
        tokio::time::sleep(Duration::from_micros(rng.gen_range(100..1200))).await;
    }

    // Get all stored averages which was calculated for each `MarkPosition` messages
    let SampleResponse::Averages(a) = sample_client2
        .request(SampleRequest::GetAverages)
        .await
        .unwrap();
    for e in a.into_iter() {
        println!("{}: {}", e.time(), e.average());
    }

    // Stop the sampler_actor first, and the msg_scheduler will be stopped gracefully.
    sampler_actor.stop().await.unwrap();
    // Stop the scheduler
    let _ = msg_scheduler.stop().await;
}

#[tokio::test]
async fn my_test() {
    main_internal().await
}
