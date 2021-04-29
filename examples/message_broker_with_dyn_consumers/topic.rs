use rand::distributions::{Distribution, Standard};
use rand::Rng;

use crate::common;

/// Possible topics
pub enum Topic {
    Info,
    Warning,
    Error,
}

/// Implementation for random generator which selects a topic
impl Distribution<Topic> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Topic {
        match rng.gen_range(0..=2) { // rand 0.8
            0 => Topic::Info,
            1 => Topic::Warning,
            _ => Topic::Error,
        }
    }
}

/// Converts enum based topic to common:Topic which is a String type
impl From<Topic> for common::Topic {
    fn from(x: Topic) -> Self {
        match x {
            Topic::Info => "info".to_string(),
            Topic::Warning => "warning".to_string(),
            Topic::Error => "error".to_string()
        }
    }
}
