extern crate async_trait;

use async_trait::async_trait;

/// Common error type
pub type Error = Box<dyn std::error::Error>;

/// Resource factory creates new resources or checks healthy of an existing.
#[async_trait]
pub trait ResourceFactory<T>: Send + Sync {
    /// Create new resource
    async fn create(&self) -> Result<T, Error>;
    /// Check healthy of existing resource
    async fn check(&self, resource: &T) -> bool;
}