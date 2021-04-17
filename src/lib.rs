//! Simple actor library
//! This library provides actor model based on asynchronous threads of `tokio` crate.
//! There are three types of actors are available
//!
//! * *message actor* - that receives and processes asynchronous messages
//! * *request actor* - that receives and processes synchronous request and replies for them
//! * *hybrid actor* - that unifies previous two types.
//! Actors can build by [`struct@crate::actor::server::actor::builder::ActorBuilder`]
pub mod common;
pub mod actor;