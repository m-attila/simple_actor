use log::{error, info};

use crate::actor::server::actor::builder::one_shot::OneShotActorBuilder;
use crate::common::StateHandler;
use crate::Res;
use crate::actor::server::actor::builder::reusable::ReusableActorBuilder;

/// Generic actor builder
pub struct ActorBuilder {
    /// receive buffer size can be set by builder
    pub(crate) receive_buffer_size: usize,
    /// Actor's name
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

    /// Create one-shot actor builder
    pub fn one_shot(self) -> OneShotActorBuilder {
        OneShotActorBuilder::new(self)
    }

    /// Create reusable actor builder
    pub fn reusable(self) -> ReusableActorBuilder {
        ReusableActorBuilder::new(self)
    }

}

/// Default state handler which used by actor, when custom handler is not set
pub(crate) struct DefaultStateHandler {}

impl DefaultStateHandler {
    pub(crate) fn new_ref() -> Box<dyn StateHandler> {
        Box::new(Self {})
    }
}

impl StateHandler for DefaultStateHandler {
    fn init(&mut self, name: String) -> Res<()> {
        info!("`{}` actor was initialized successfully", name);
        Ok(())
    }

    fn terminate(&mut self, name: String, reason: &Res<()>) {
        if reason.is_ok() {
            info!("{} actor terminated with result: {:?}", name, reason)
        } else {
            error!("{} actor terminated with error: {:?}", name, reason)
        }
    }
}