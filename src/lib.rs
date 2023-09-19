//! ## Simple actor library
//!
//! This library provides actor model implementation which is based on `tokio` crate. This model was
//! inspired by `gen_server` module of `Erlang` ecosystem. `gen_server` is an actor which
//! receives and processes commands, which could change actor's internal state during the execution.
//! A command is either a *message* or a *request*. A message is processed by asynchronously, but the request is synchronous.
//! Synchronous processing means that the caller must wait for the end of the message processing. If a caller sends an asynchronous message
//! it continues its running without any waiting.
//!
//! ### About command processing
//!
//! Actor can process only one command at once, in other words it serializes received commands. But the internal command
//! processing could be asynchronous too, if this processing occurs within a task which is started by either `tokio::task::spawn_blocking`
//! or `tokio::spawn` methods. In this case the actor could deal with the next command before the previous one has been finished.
//! When such a task execution has been finished, the actor sends its result as another request to itself.
//! This self-request can be synchronous which could change the actor state and after changing as a reply could send back to the client.
//! Or this self-request can be asynchronous again, to start a new task for a new computation.
//!
//! #### There are three types of actors are available
//!
//! * *message actor* processes asynchronous messages,
//! * *request actor* processes synchronous request,
//! * *hybrid actor* unifies previous two types.
//!
//! # Build an actor
//!
//! Actors can build by [`ActorBuilder`](struct@crate::ActorBuilder).
//! The builder has several methods to build different types of actors.
//!
//! ## MessageActor
//!
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such an actor which can receive only messages.
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
//! #[derive(Default)]
//! struct TestActor(i32);
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
//! /// Implements how the actor handles messages
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
//!         // upper bound of the number of messages
//!         .receive_buffer_size(128)
//!         // builder can build only one actor at once
//!         .one_shot()
//!         // set the actor's type and its command handler
//!         .message_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create a client for the actor
//!     let actor_client = actor.client();
//!
//!     // Send an `increment` message
//!     actor_client.message(ActorMessages::Inc(10)).await.unwrap();
//!     // Send a `decrement` message
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
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such an actor which can receive only requests.
//!
//! ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::{RequestHandler, Res};
//! use simple_actor::ActorBuilder;
//!
//! // ..Put test actor implementation here from the previous example...
//! # #[derive(Default)]
//! # struct TestActor(i32);
//!
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
//! /// Implements how the actor handles requests
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
//!                 // Return the response
//!                 Ok(ActorResponses::Success)
//!             }
//!             // Get counter value
//!             ActorRequests::Get => {
//!                 // Return the response
//!                 Ok(ActorResponses::Counter(self.0))
//!             }
//!         }
//!     }
//! }
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create a new actor with the builder
//!     let actor = ActorBuilder::new()
//!         // actor's name
//!         .name("test_actor")
//!         // upper bound of the number of messages
//!         .receive_buffer_size(128)
//!         // builder can build only one actor at once
//!         .one_shot()
//!         // set the actor's type and its command handler
//!         .request_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create a client for the actor
//!     let actor_client = actor.client();
//!
//!     // Send an `increment` request and wait for the response
//!     assert_eq!(ActorResponses::Success, actor_client.request(ActorRequests::Inc(10)).await.unwrap());
//!     // Send a `get` request and wait for the response
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
//! * [`ActorBuilder`](struct@crate::ActorBuilder) creates such an actor which can receive messages and requests as well.
//!
//! ### Example
//!
//! ```
//!
//! use async_trait::async_trait;
//! use simple_actor::ActorBuilder;
//! use simple_actor::{MessageHandler, RequestHandler, Res};
//!
//! // ...Put test actor implementation here from the first example...
//! /// Available messages
//! # #[derive(Debug)]
//! # enum ActorMessages{
//! #    /// Increment the counter
//! #    Inc(i32),
//! #    /// Decrement the counter
//! #    Dec(i32),
//! # }
//! # #[derive(Default)]
//! # struct TestActor(i32);
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
//! /// Implements how the actor handles messages
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
//!         // Returns the counter's value
//!         Ok(ActorResponses::Value(self.0))
//!     }
//! }
//!
//!
//! #[tokio::main]
//! pub async fn main() {
//!     // Create a new actor with the builder
//!     let actor = ActorBuilder::new()
//!         // actor's name (optional)
//!         .name("test_actor")
//!         // upper bound of the number of messages
//!         .receive_buffer_size(128)
//!         // builder can build only one actor at once
//!         .one_shot()
//!         // set actor type and its handler
//!         .hybrid_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create a client for actor
//!     let actor_client = actor.client();
//!
//!     // Send an `increment` message
//!     actor_client.message(ActorMessages::Inc(10)).await.unwrap();
//!     // Send a `decrement` message
//!     actor_client.message(ActorMessages::Dec(5)).await.unwrap();
//!     // Get the counter's value
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
//! In this crate there is a scheduler mechanism, which can work together with those actors can receive messages.
//! The [`MessageScheduler`](struct@crate::MessageScheduler) periodically sends messages into the actor which belongs to the scheduler.
//! A message actor can have more schedulers at a time.
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
//! # #[derive(Default)]
//! # struct TestActor(i32);
//!
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
//!     use simple_actor::Scheduling;
//!     // Create a new actor with the builder
//!
//!     let actor = ActorBuilder::new()
//!         // actor's name (optional)
//!         .name("test_actor")
//!         // upper bound of the number of messages
//!         .receive_buffer_size(128)
//!         // builder can build only one actor at once
//!         .one_shot()
//!         // set actor type and it's handler
//!         .message_actor(Box::new(TestActor::default()))
//!         // build the actor
//!         .build();
//!
//!     // Create a client for actor
//!     let actor_client_1 = actor.client();
//!     // Create another client for the actor
//!     let actor_client_2 = actor.client();
//!
//!     // In every 30 milliseconds the scheduler sends an `increment by 10 message` to the actor
//!     let message_scheduler_1 = MessageScheduler::new(ActorMessages::Inc(10), Scheduling::Periodic(Duration::from_millis(30)), actor_client_1);
//!     // In every 60 milliseconds the scheduler sends a `decrement by 5 message` to the actor
//!     let message_scheduler_2 = MessageScheduler::new(ActorMessages::Dec(5), Scheduling::Periodic(Duration::from_millis(60)), actor_client_2);
//!
//!     // Stop the actor first
//!     actor.stop().await.unwrap();
//!     // Stop the schedulers. Schedulers will returns with error, because actor already has stopped.
//!     message_scheduler_1.stop().await.unwrap_err();
//!     message_scheduler_2.stop().await.unwrap_err();
//! }
//! ```
//! #### Other examples
//!
//! * `heavy_computation`
//!
//! It introduces how can execute asynchronous request, and how the request can modify the actor's internal state
//! according its execution result.
//!
//! * `message_broker_with_actor_consumers`
//!
//! It introduces a public-subscribe messaging example for several topics, where the topic handler and
//! subscribers are actors.
//!
//! * `message_broker_with_dyn_consumers`
//!
//! It introduces a public-subscribe messaging example for several topics, similarly as in the previous example,
//! but only the topic handler is an actor, clients are simple dynamic trait implementations.
//!
//! * `resource_pool`
//!
//! This example introduces a resource pool handler, where clients send their callback functions to the actor.
//! These callbacks can execute any asynchronous operation, which works an allocated resource from the pool.
//! The resource pool handles such resources which are usable either permanently or temporarily.
//! Usually the pool keeps only permanent resources but when the load in increasing, on demand creates temporary ones.
//! After the load goes down and the temporary resources are idle, they will be disposed.
//!
//! * `sampler`
//!
//! This example shows how schedulers can be attached to actors. Schedulers transform timer events into messages
//! which can be handled in the actors.
//!
//! ## Logging
//!
//! Simple actor was integrated with [`log`](https://!crates.io/crates/log) crate.

pub mod actor;
pub mod common;

pub use crate::actor::client::hybrid::ActorHybridClient;
pub use crate::actor::client::message::ActorMessageClient;
pub use crate::actor::client::request::ActorRequestClient;
pub use crate::actor::client::scheduler::common::Scheduling;
pub use crate::actor::client::scheduler::message::MessageScheduler;
pub use crate::actor::client::sync::message::MessageActorNotify;
pub use crate::actor::server::actor::builder::common::ActorBuilder;
pub use crate::common::{ActorError, MessageHandler, RequestHandler, Res};
