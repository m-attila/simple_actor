extern crate async_trait;

use async_trait::async_trait;

use simple_actor::actor::server::actor::hybrid::HybridActor;
use simple_actor::common::{MessageHandler, RequestHandler, Res};
use simple_actor::ActorBuilder;

use crate::common::{Average, Measurement, SampleMessages, SampleRequest, SampleResponse};

/// Maximum number of stored average values.
const MAX_STORED_AVERAGES: usize = 128;

/// Sampler logic
struct Sampler {
    /// Count of measurement data which was received since last time markup
    counter: u128,
    /// Summary of measurement data
    sums: u128,
    /// The stored averages on each time markup
    averages: Vec<Average>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            counter: 0,
            sums: 0,
            averages: Vec::with_capacity(128),
        }
    }
}

impl Sampler {
    /// Store new measurement data
    fn store(&mut self, data: Measurement) {
        self.counter += 1;
        self.sums += data as u128;
    }

    /// Calculate a new average from those measurements which were received since the previous
    /// time markup, and start a new averaging process
    fn mark_position(&mut self) {
        if self.averages.len() == MAX_STORED_AVERAGES {
            self.averages.remove(0);
        }
        self.averages.push(self.average());
        self.reset();
    }

    /// Return all average data since the actor was started
    pub fn averages(&mut self) -> Vec<Average> {
        std::mem::replace(self.averages.as_mut(), Vec::with_capacity(128))
    }

    /// Calculate current average data
    fn average(&self) -> Average {
        let average = if self.counter == 0 {
            0
        } else {
            (self.sums / self.counter) as Measurement
        };
        Average::new(average)
    }

    /// Reset data
    fn reset(&mut self) {
        self.counter = 0;
        self.sums = 0;
    }
}

/// Actor for Sampler
pub struct SamplerActor {
    /// Sampler logic
    sampler: Sampler,
    /// Receive buffer size
    buffer_size: usize,
}

impl SamplerActor {
    /// Create a new actor with a given receive buffer size
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            sampler: Sampler::default(),
        }
    }

    /// Start the actor
    pub fn start(self) -> HybridActor<SampleMessages, SampleRequest, SampleResponse> {
        ActorBuilder::new()
            .name("sampler")
            .receive_buffer_size(self.buffer_size)
            .one_shot()
            .hybrid_actor(Box::new(self))
            .build()
    }
}

#[async_trait]
impl MessageHandler for SamplerActor {
    type Message = SampleMessages;

    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        match message {
            // Handle measurement data
            SampleMessages::Store(m) => self.sampler.store(m),
            // Handle the markup command
            SampleMessages::MarkPosition => self.sampler.mark_position(),
        }
        Ok(())
    }
}

#[async_trait]
impl RequestHandler for SamplerActor {
    type Request = SampleRequest;
    type Reply = SampleResponse;

    async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply> {
        // Handle `GetAverages` request
        Ok(SampleResponse::Averages(self.sampler.averages()))
    }
}
