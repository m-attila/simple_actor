//! Simple actor library
//!
//! This library provides actor model implementation which based on `tokio` crate. This model was
//! inspired by `gen_server` module of `Erlang` ecosystem. `gen_server` is an actor which
//! receives and processes serial of commands, which could change its internal state during the execution.
//! The commands can be *messages* and *requests*. Message is processed by asynchronously, independent from
//! caller's running.
//! The request is processed by synchronously, the caller waits for its result. If the internal processing of the request
//! is synchronous, actor could not receive the next command until the previous has finished. The internal request
//! processing could be asynchronous too, when it starts a new task with the `tokio::task::spawn_blocking` or `tokio::spawn` methods,
//! so the actor could go for the next command during its execution. When the thread execution has finished,
//! its result will be send by an other request to the actor again. This new request can be synchronous
//! which could change the actor state, and this, as a reply could go back to the client, as the original request's result.
//! If the new request is asynchronous again, it will start a new thread for a new computation.
//!
//! ### There are three types of actors are available
//!
//! * *message actor* - that processes asynchronous messages,
//! * *request actor* - that processes synchronous request,
//! * *hybrid actor* - that unifies previous two types.
//!
//! # Build an actor
//!
//! Actors can build by [`ActorBuilder`](struct@crate::ActorBuilder)
//! The builder has several method to build different types of actors.
//!
//! ## MessageActor
//!
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such actor
//! which can receive only messages
//!
//! ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::{MessageHandler, Res};
//! use simple_actor::ActorBuilder;
//!
//! /// Actor which contains a counter
//! struct TestActor(i32);
//!
//! impl Default for TestActor{
//!     fn default() -> Self {
//!         Self(0)
//!     }
//! }
//!
//! /// Available messages
//! #[derive(Debug)]
//! enum ActorMessages{
//!     /// Increment the counter
//!     Inc(i32),
//!     /// Decrement the counter
//!     Dec(i32),
//! }
//!
//! /// Implements how does the actor handle the messages
//! #[async_trait]
//! impl MessageHandler for TestActor{
//!     // Type of actor messages
//!     type Message = ActorMessages;
//!
//!     async fn process_message(&mut self, message: Self::Message) -> Res<()> {
//!         match message{
//!             // Increment the counter
//!             ActorMessages::Inc(val) => self.0 += val,
//!             // Decrement the counter
//!             ActorMessages::Dec(val) => self.0 -= val
//!         };
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create new actor with the builder
//!     let actor = ActorBuilder::new()
//!         // actor's name (optional)
//!         .name("test_actor")
//!         // maximal size of the the messages' receiving buffer
//!         .receive_buffer_size(128)
//!         // builder could be use only once
//!         .one_shot()
//!         // set actor type and handler
//!         .message_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create client for actor
//!     let actor_client = actor.client();
//!
//!     // Send increment message
//!     actor_client.message(ActorMessages::Inc(10)).await.unwrap();
//!     // Send decrement message
//!     actor_client.message(ActorMessages::Dec(5)).await.unwrap();
//!
//!     // Stop the actor
//!     actor.stop().await.unwrap();
//! }
//!
//! ```
//!
//! ## RequestActor
//!
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such actor
//! which can receive only requests
//!
//! ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::{RequestHandler, Res};
//! use simple_actor::ActorBuilder;
//!
//! // ...Test actor implementation from the previous example...
//! # struct TestActor(i32);
//! # impl Default for TestActor{
//! #      fn default() -> Self {
//! #          Self(0)
//! #      }
//! # }
//! /// Available requests
//! #[derive(Debug)]
//! enum ActorRequests {
//!     /// Increment the counter
//!     Inc(i32),
//!     /// Get counter value
//!     Get
//! }
//!
//! /// Available responses
//! #[derive(Debug, PartialEq)]
//! enum ActorResponses{
//!     /// Response for `ActorRequests::Inc`
//!     Success,
//!     /// Response for `ActorRequests::Get`
//!     Counter(i32),
//! }
//!
//! /// Implements how does the actor handle the requests
//! #[async_trait]
//! impl RequestHandler for TestActor{
//!     // Type of the requests
//!     type Request = ActorRequests;
//!     // Type of the Responses
//!     type Reply = ActorResponses;
//!
//!     async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
//!         match request {
//!             // Increment the counter
//!             ActorRequests::Inc(inc) => {
//!                 self.0 += inc;
//!                 Ok(ActorResponses::Success)
//!             }
//!             // Get counter value
//!             ActorRequests::Get => {
//!                 Ok(ActorResponses::Counter(self.0))
//!             }
//!         }
//!     }
//! }
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create new actor with the builder
//!     let actor = ActorBuilder::new()
//!         // actor's name
//!         .name("test_actor")
//!         // the maximal size of the request receiving buffer
//!         .receive_buffer_size(128)
//!         // builder could be use only once
//!         .one_shot()
//!         // set actor type and handler
//!         .request_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create client for actor
//!     let actor_client = actor.client();
//!
//!     // Send increment request and wait for the response
//!     assert_eq!(ActorResponses::Success, actor_client.request(ActorRequests::Inc(10)).await.unwrap());
//!     // Send get request and wait for the response
//!     assert_eq!(ActorResponses::Counter(10), actor_client.request(ActorRequests::Get).await.unwrap());
//!
//!     // Stop the actor
//!     actor.stop().await.unwrap();
//! }
//!
//! ```
//!
//! ## HybridActor
//!
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such actor
//! which can receive both messages and requests
//!
//! ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::ActorBuilder;
//! use simple_actor::{MessageHandler, RequestHandler, HybridHandler, Res};
//!
//! // ...Test actor implementation from the messages actor's example...
//! # struct TestActor(i32);
//! # impl Default for TestActor{
//! #     fn default() -> Self {
//! #         Self(0)
//! #     }
//! # }
//! # #[derive(Debug)]
//! # enum ActorMessages{
//! #     /// Increment the counter
//! #     Inc(i32),
//! #     /// Decrement the counter
//! #     Dec(i32),
//! #  }
//! /// Available requests
//! #[derive(Debug)]
//! enum ActorRequests{
//!     /// Get the counter's value
//!     Get
//! }
//!
//! /// Available responses
//! #[derive(Debug, PartialEq)]
//! enum ActorResponses{
//!     /// The counter value
//!     Value(i32)
//! }
//!
//! /// Implements how does the actor handle the messages
//! #[async_trait]
//! impl MessageHandler for TestActor{
//!     // Type of actor messages
//!     type Message = ActorMessages;
//!
//!     async fn process_message(&mut self, message: Self::Message) -> Res<()> {
//!         match message{
//!             // Increment the counter
//!             ActorMessages::Inc(val) => self.0 += val,
//!             // Decrement the counter
//!             ActorMessages::Dec(val) => self.0 -= val
//!         };
//!         Ok(())
//!     }
//! }
//!
//! /// Implements how does the actor handle the requests
//! #[async_trait]
//! impl RequestHandler for TestActor{
//!     // Type of the requests
//!     type Request = ActorRequests;
//!     // Type of the Responses
//!     type Reply = ActorResponses;
//!
//!     async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
//!         // Returns with the counter's value
//!         Ok(ActorResponses::Value(self.0))
//!     }
//! }
//!
//! /// Actor accepts both messages and requests
//! impl HybridHandler for TestActor{
//!     // Some boilerplate code to help conversion from HybridHandler into RequestHandler
//!     fn request_handler_ref(&self) -> &dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
//!         self
//!     }
//!
//!     // Some boilerplate code to help conversion from HybridHandler into RequestHandler
//!     fn request_handler_mut(&mut self) -> &mut dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
//!         self
//!     }}
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create new actor with the builder
//!     let actor = ActorBuilder::new()
//!         // actor's name (optional)
//!         .name("test_actor")
//!         // the maximal size of receiving buffer
//!         .receive_buffer_size(128)
//!         // builder could be use only once
//!         .one_shot()
//!         // set actor type and handler
//!         .hybrid_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create client for actor
//!     let actor_client = actor.client();
//!
//!     // Send increment message
//!     actor_client.message(ActorMessages::Inc(10)).await.unwrap();
//!     // Send decrement message
//!     actor_client.message(ActorMessages::Dec(5)).await.unwrap();
//!     // Get counter's value
//!     assert_eq!(ActorResponses::Value(10 - 5), actor_client.request(ActorRequests::Get).await.unwrap());
//!
//!     // Stop the actor
//!     actor.stop().await.unwrap();
//! }
//!
//! ```
//!
//! # Scheduling messages
//!
//! In the crate there is a scheduler mechanism, it can work together with those actors, which can receive messages.
//! The [`MessageScheduler`](struct@crate::MessageScheduler) periodically sends messages into the actor which belongs to the scheduler.
//! One message actor may has more than one schedulers.
//!
//!  ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::{ActorBuilder, MessageScheduler, Scheduling, MessageHandler, Res};
//! use tokio::time::Duration;
//!
//! # /// Actor which contains a counter
//! # struct TestActor(i32);
//! # impl Default for TestActor{
//! #    fn default() -> Self {
//! #        Self(0)
//! #    }
//! # }
//! # /// Available messages
//! # #[derive(Debug, Clone)]
//! # enum ActorMessages{
//! #     /// Increment the counter
//! #     Inc(i32),
//! #     /// Decrement the counter
//! #     Dec(i32),
//! # }
//! # /// Implements how does the actor handle the messages
//! #[async_trait]
//! impl MessageHandler for TestActor{
//!     // Type of actor messages
//!     type Message = ActorMessages;
//!
//!     async fn process_message(&mut self, message: Self::Message) -> Res<()> {
//!         match message{
//!             // Increment the counter
//!             ActorMessages::Inc(val) => self.0 += val,
//!             // Decrement the counter
//!             ActorMessages::Dec(val) => self.0 -= val
//!         };
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create new actor with the builder
//!
//! use simple_actor::Scheduling;
//! let actor = ActorBuilder::new()
//!         // actor's name (optional)
//!         .name("test_actor")
//!         // the message receiving buffer maximal size
//!         .receive_buffer_size(128)
//!         // builder could be use only once
//!         .one_shot()
//!         // set actor type and handler
//!         .message_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create client for actor
//!     let actor_client_1 = actor.client();
//!     // Create client for actor
//!     let actor_client_2 = actor.client();
//!
//!     // Every 30 milliseconds sends an increment by 10 message to the actor
//!     let message_scheduler_1 = MessageScheduler::new(ActorMessages::Inc(10), Scheduling::Periodic(Duration::from_millis(30)), actor_client_1);
//!     // Every 60 milliseconds sends a decrement by 5 message to the actor
//!     let message_scheduler_2 = MessageScheduler::new(ActorMessages::Dec(5), Scheduling::Periodic(Duration::from_millis(60)), actor_client_2);
//!
//!     // Stop the actor first
//!     actor.stop().await.unwrap();
//!     // Stop the schedulers. Schedulers will returns with error, because actor already has stopped.
//!     message_scheduler_1.stop().await.unwrap_err();
//!     message_scheduler_2.stop().await.unwrap_err();
//! }
//! ```
//!
//! ### Other examples
//!
//! * `heavy_computation`
//! Introduces how can be execute asynchronous request, and how can they modify the actor's state
//! depend of their execution results.
//! * `message_broker_with_actor_consumers`
//! Introduces a public-subscribe messaging example for several topics, where the topic handler and the
//! subscribers are actors.
//! * `message_broker_with_dyn_consumers`
//! Introduces a public-subscriber messaging example for several topics, similary as the previous example,
//! but only the topic handler is an actor, the clients are simple dynamic trait implementations.
//! * `resource_pool`
//! This example introduces a resource pool handler, where the clients send their callback functions to the actor.
//! This functions generate an asycnhronous operation, which uses the allocated resource. The resource pool handles
//! a set of resources which can be permanent and temporary ones. The pool keeps the permanent resources and
//! when the load in increasing, it creates temporary ones, with a given idle time threshold. After the load goes down,
//! and the idle time has elapsed, temporary resources will be dropped.
//! * `sampler`
//! This example shows, how can be attach schedulers to the actors. The schedulers transform timer events
//! into messages which can be handled in the actors.
//!
//! # Logging
//!
//! Simple actor was integrated with [`log`](https://crates.io/crates/log) crate.
//!

pub mod common;
pub mod actor;

pub use crate::common::{MessageHandler, RequestHandler, HybridHandler, Res, ActorError};
pub use crate::actor::server::actor::builder::common::ActorBuilder;
pub use crate::actor::client::scheduler::common::Scheduling;
pub use crate::actor::client::scheduler::message::MessageScheduler;
pub use crate::actor::client::message::ActorMessageClient;
pub use crate::actor::client::request::ActorRequestClient;
pub use crate::actor::client::hybrid::ActorHybridClient;
pub use crate::actor::client::sync::message::MessageActorNotify;