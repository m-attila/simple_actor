//! This example introduces how can be generate and handle periodically scheduled messages in actor.
//! The `SamplerActor` receives measurement data and calculate average value of them within a given
//! time window. This calculation is performed for a periodically generated `MarkPosition` message.
//! The actor stores all average calculations, and give them back for the `GetAverages` requests.
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
    // Create SampleActor instance
    let sampler = SamplerActor::new(256);
    // Start actor
    let sampler_actor = sampler.start();

    // Create client to send measurement data
    let sample_client = sampler_actor.client();
    // Create client to send periodically scheduled `MarkPosition` message
    let sample_client2 = sampler_actor.client();
    // Start message scheduler
    let msg_scheduler = MessageScheduler::new(SampleMessages::MarkPosition,
                                              Scheduling::Periodic(Duration::from_millis(10)),
                                              sampler_actor.message_client());

    let mut rng = rand::thread_rng();
    // Send 1000 measurement values to the actor
    for _ in 0..1000 {
        let value = rng.gen_range(0..1000);
        sample_client.message(SampleMessages::Store(value)).await.unwrap();
        // Wait some microseconds between the message sending
        tokio::time::sleep(Duration::from_micros(rng.gen_range(100..1200))).await;
    }

    // Get all stored averages which was calculated for each `MarkPosition` messages
    let SampleResponse::Averages(a) = sample_client2.request(SampleRequest::GetAverages).await.unwrap();
    for e in a.into_iter() {
        println!("{}: {}", e.time(), e.average());
    }

    // Stop the sampler_actor first, and the msg_scheduler will be stopped gracefully.
    sampler_actor.stop().await.unwrap();
    // Stop the scheduler
    msg_scheduler.stop().await.unwrap();
}