use log::{error, info};

use crate::actor::server::actor::builder::one_shot::OneShotActorBuilder;
use crate::actor::server::actor::builder::reusable::ReusableActorBuilder;
use crate::common::StateHandler;
use crate::Res;

/// Generic actor builder
pub struct ActorBuilder {
    /// receive buffer size can be set by builder
    pub(crate) receive_buffer_size: usize,
    /// actor's name
    pub(crate) name: String,
}

impl Default for ActorBuilder {
    fn default() -> Self {
        Self {
            receive_buffer_size: 32,
            name: String::default(),
        }
    }
}

impl ActorBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set receive buffer size
    pub fn receive_buffer_size(mut self, receive_buffer_size: usize) -> Self {
        self.receive_buffer_size = receive_buffer_size;
        self
    }

    /// Set the actor's name
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Create one-shot actor builder
    pub fn one_shot(self) -> OneShotActorBuilder {
        OneShotActorBuilder::new(self)
    }

    /// Create reusable actor builder
    pub fn reusable(self) -> ReusableActorBuilder {
        ReusableActorBuilder::new(self)
    }
}

/// Default state handler which used by actor, when the custom handler has not set
pub(crate) struct DefaultStateHandler {}

impl DefaultStateHandler {
    pub(crate) fn new_ref() -> Box<dyn StateHandler> {
        Box::new(Self {})
    }
}

impl StateHandler for DefaultStateHandler {
    fn init(&mut self, name: String) -> Res<()> {
        info!("`{}` actor has initialized successfully", name);
        Ok(())
    }

    fn terminate(&mut self, name: String, reason: &Res<()>) {
        if reason.is_ok() {
            info!("{} actor has terminated with result: {:?}", name, reason)
        } else {
            error!("{} actor has terminated with error: {:?}", name, reason)
        }
    }
}
