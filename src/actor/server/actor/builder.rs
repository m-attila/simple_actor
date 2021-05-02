use log::{info, error};
use crate::actor::server::actor::hybrid::HybridActor;
use crate::actor::server::actor::message::MessageActor;
use crate::actor::server::actor::request::RequestActor;
use crate::common::{HybridHandler, MessageHandler, RequestHandler, Res, StateHandler};
use std::fmt::Debug;

/// Builder pattern to create and start new actors
pub struct ActorBuilder {
    /// Optionals state handler that can be set by builder
    state_handler: Option<Box<dyn StateHandler>>,
    /// receive buffer size can be set by builder
    receive_buffer_size: usize,
    /// Actor's name
    name: String,
}

impl Default for ActorBuilder {
    fn default() -> Self {
        Self {
            state_handler: None,
            receive_buffer_size: 32,
            name: String::default(),
        }
    }
}

impl ActorBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets state handler's custom implementation
    pub fn state_handler(mut self, state_handler: Box<dyn StateHandler>) -> Self {
        self.state_handler = Some(state_handler);
        self
    }

    /// Sets receive buffer size
    pub fn receive_buffer_size(mut self, receive_buffer_size: usize) -> Self {
        self.receive_buffer_size = receive_buffer_size;
        self
    }

    /// Sets the actor's name
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Builds message handling actor with given message processor handler
    pub fn build_message_actor<ME>(self, msg_handler: Box<dyn MessageHandler<Message=ME>>) -> MessageActor<ME>
        where ME: 'static + Send + Debug{
        MessageActor::new(self.name, msg_handler,
                          self.state_handler
                              .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }

    /// Builds request handler actor with given request processor handler
    pub fn build_request_actor<MR, R>(self, req_handler: Box<dyn RequestHandler<Request=MR, Reply=R>>) -> RequestActor<MR, R>
        where MR: 'static + Send + Debug,
              R: 'static + Send + Debug{
        RequestActor::new(self.name, req_handler,
                          self.state_handler
                              .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }

    /// Builds message and request handler actor with given handler
    pub fn build_hybrid_actor<ME, MR, R>(self,
                                         handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>) -> HybridActor<ME, MR, R>
        where ME: 'static + Send + Debug,
              MR: 'static + Send + Debug,
              R: 'static + Send + Debug{
        HybridActor::new(self.name, handler,
                         self.state_handler
                             .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }
}

/// Default state handler which used by actor, when custom handler is not set
struct DefaultStateHandler {}

impl StateHandler for DefaultStateHandler {
    fn init(&mut self, name: String) -> Res<()> {
        info!("`{}` actor was initialized successfully", name);
        Ok(())
    }

    fn terminate(&mut self, name: String, reason: &Res<()>) {
        if reason.is_ok() {
            info!("{} actor terminated with result: {:?}", name, reason)
        }
        else{
            error!("{} actor terminated with error: {:?}", name, reason)
        }
    }
}