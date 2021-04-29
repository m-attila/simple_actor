extern crate chrono;

use self::chrono::{DateTime, Utc};

/// Measurement values which received by sampler
pub type Measurement = u32;

/// One result which contains the average values from the marked position
#[derive(Debug)]
pub struct Average {
    /// Time markup
    pub time: DateTime<Utc>,
    /// Averages of the measurements
    pub average: Measurement,
}

impl Average {
    /// Creates new instance
    pub fn new(average: Measurement) -> Self {
        Self {
            time: Utc::now(),
            average,
        }
    }

    /// Returns the timestamp markup
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    /// Returns the average value
    pub fn average(&self) -> Measurement {
        self.average
    }
}

#[derive(Debug, Clone)]
/// Messages which received by sampler
pub enum SampleMessages {
    /// Stores measurement value
    Store(Measurement),
    /// Scheduled message. The received measurement data will be averaged between two `MarkPosition`
    /// time markups.
    MarkPosition,
}

#[derive(Debug)]
pub enum SampleRequest {
    /// Return all averages which was stored in the sampler
    GetAverages,
}

#[derive(Debug)]
pub enum SampleResponse {
    /// Response of `SampleRequest::GetAverages` request. Contains all averages which was collected
    /// on each time markups.
    Averages(Vec<Average>)
}
