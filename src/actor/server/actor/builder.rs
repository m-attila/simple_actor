use crate::actor::server::actor::hybrid::HybridActor;
use crate::actor::server::actor::message::MessageActor;
use crate::actor::server::actor::request::RequestActor;
use crate::common::{HybridHandler, MessageHandler, RequestHandler, Res, StateHandler};

/// Actor builder
pub struct ActorBuilder {
    /// Optionals state handler that can be set by builder
    state_handler: Option<Box<dyn StateHandler>>,
    /// receive buffer size that can be set by builder
    receive_buffer_size: usize,
}

impl Default for ActorBuilder {
    fn default() -> Self {
        Self {
            state_handler: None,
            receive_buffer_size: 32,
        }
    }
}

impl ActorBuilder {
    pub fn new() -> Self {
        Default::default()
    }
    /// Sets state handler custom implementation
    pub fn state_handler(&mut self, state_handler: Box<dyn StateHandler>) -> &mut Self {
        self.state_handler = Some(state_handler);
        self
    }

    /// Sets receive buffer size for messagebox
    pub fn receive_buffer_size(&mut self, receive_buffer_size: usize) -> &mut Self {
        self.receive_buffer_size = receive_buffer_size;
        self
    }

    /// Builds message handling actor with given message processor implementation
    pub fn build_message_actor<ME>(self, msg_handler: Box<dyn MessageHandler<Message=ME>>) -> MessageActor<ME>
        where ME: 'static + Send {
        MessageActor::new(msg_handler,
                          self.state_handler
                              .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }

    /// Builds request handler actor with given request processor implementation
    pub fn build_request_actor<MR, R>(self, req_handler: Box<dyn RequestHandler<Request=MR, Reply=R>>) -> RequestActor<MR, R>
        where MR: 'static + Send,
              R: 'static + Send {
        RequestActor::new(req_handler,
                          self.state_handler
                              .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }

    /// Builds message and request handler actor with given implementation
    pub fn build_hybrid_actor<ME, MR, R>(self,
                                         handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>) -> HybridActor<ME, MR, R>
        where ME: 'static + Send,
              MR: 'static + Send,
              R: 'static + Send {
        HybridActor::new(handler,
                         self.state_handler
                             .unwrap_or(Box::new(DefaultStateHandler {})), self.receive_buffer_size)
    }
}

/// Default state handler which used by actor when custom handler has not given
struct DefaultStateHandler {}

impl StateHandler for DefaultStateHandler {
    fn init(&mut self) -> Res<()> {
        println!("Initialized");
        Ok(())
    }

    fn terminate(&mut self, reason: &Res<()>) {
        println!("Terminated with result: {:?}", reason);
    }
}