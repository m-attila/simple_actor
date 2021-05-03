use std::fmt::Debug;

use crate::actor::server::actor::builder::common::DefaultStateHandler;
use crate::actor::server::actor::hybrid::HybridActor;
use crate::actor::server::actor::message::MessageActor;
use crate::actor::server::actor::request::RequestActor;
use crate::ActorBuilder;
use crate::common::{HybridHandler, MessageHandler, RequestHandler, StateHandler};

/// Reusable actor builder. The build process could be execute more than once.
pub struct ReusableActorBuilder {
    /// Generic builder
    builder: ActorBuilder,
    /// Optionals state handler generator that can be set by builder
    state_handler: Option<Box<dyn Fn() -> Box<dyn StateHandler>>>,
}

impl ReusableActorBuilder {
    /// Create new instance
    pub fn new(builder: ActorBuilder) -> Self {
        Self {
            builder,
            state_handler: None,
        }
    }

    /// Sets state handler's custom implementation
    pub fn state_handler(mut self, state_handler: impl Fn() -> Box<dyn StateHandler> + 'static) -> Self {
        self.state_handler = Some(Box::new(state_handler));
        self
    }

    /// Create message actor builder
    pub fn message_actor<ME>(self, msg_handler: impl Fn() -> Box<dyn MessageHandler<Message=ME>> + 'static) -> MessageActorBuilder<ME>
        where ME: 'static + Send + Debug {
        MessageActorBuilder::new(self, msg_handler)
    }

    /// Create request actor builder
    pub fn request_actor<MR, R>(self, req_handler: impl Fn() -> Box<dyn RequestHandler<Request=MR, Reply=R>> + 'static) -> RequestActorBuilder<MR, R>
        where MR: 'static + Send + Debug,
              R: 'static + Send + Debug {
        RequestActorBuilder::new(self, req_handler)
    }

    /// Create hybrid actor builder
    pub fn hybrid_actor<ME, MR, R>(self,
                                   handler: impl Fn() -> Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>> + 'static) -> HybridActorBuilder<ME, MR, R>
        where ME: 'static + Send + Debug,
              MR: 'static + Send + Debug,
              R: 'static + Send + Debug {
        HybridActorBuilder::new(self, handler)
    }
}


/// Builder for message actors
pub struct MessageActorBuilder<ME> {
    builder: ReusableActorBuilder,
    handler: Box<dyn Fn() -> Box<dyn MessageHandler<Message=ME>>>,
}

impl<ME> MessageActorBuilder<ME>
    where
        ME: 'static + Send + Debug {
    /// Create new instance
    fn new(builder: ReusableActorBuilder, handler: impl Fn() -> Box<dyn MessageHandler<Message=ME>> + 'static) -> Self {
        Self {
            builder,
            handler: Box::new(handler),
        }
    }

    /// Start new message actor
    pub fn build(&self) -> MessageActor<ME> {
        MessageActor::new(self.builder.builder.name.clone(),
                          (self.handler)(),
                          self.builder.state_handler.as_ref()
                              .map_or(DefaultStateHandler::new_ref(), |f| f()),
                          self.builder.builder.receive_buffer_size)
    }
}

/// Builder for request actors
pub struct RequestActorBuilder<MR, R> {
    builder: ReusableActorBuilder,
    handler: Box<dyn Fn() -> Box<dyn RequestHandler<Request=MR, Reply=R>>>,
}

impl<MR, R> RequestActorBuilder<MR, R>
    where
        MR: 'static + Send + Debug,
        R: 'static + Send + Debug {
    /// Create new instance
    fn new(builder: ReusableActorBuilder, handler: impl Fn() -> Box<dyn RequestHandler<Request=MR, Reply=R>> + 'static) -> Self {
        Self {
            builder,
            handler: Box::new(handler),
        }
    }

    /// Build and start actor
    pub fn build(&self) -> RequestActor<MR, R> {
        RequestActor::new(self.builder.builder.name.clone(),
                          (self.handler)(),
                          self.builder.state_handler.as_ref()
                              .map_or(DefaultStateHandler::new_ref(), |f| f()),
                          self.builder.builder.receive_buffer_size)
    }
}

/// Builder for hybrid actors
pub struct HybridActorBuilder<ME, MR, R> {
    builder: ReusableActorBuilder,
    handler: Box<dyn Fn() -> Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>>,
}

impl<ME, MR, R> HybridActorBuilder<ME, MR, R>
    where
        ME: 'static + Send + Debug,
        MR: 'static + Send + Debug,
        R: 'static + Send + Debug {
    /// Create new instance
    fn new(builder: ReusableActorBuilder, handler: impl Fn() -> Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>> + 'static) -> Self {
        Self {
            builder,
            handler: Box::new(handler),
        }
    }

    /// Build and start hybrid actor
    pub fn build(&self) -> HybridActor<ME, MR, R> {
        HybridActor::new(self.builder.builder.name.clone(),
                         (self.handler)(),
                         self.builder.state_handler.as_ref()
                             .map_or(DefaultStateHandler::new_ref(), |f| f()),
                         self.builder.builder.receive_buffer_size)
    }
}
