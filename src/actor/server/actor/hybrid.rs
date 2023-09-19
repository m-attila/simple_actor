use std::fmt::Debug;

use log::{debug, info};

use crate::actor::client::hybrid::ActorHybridClient;
use crate::actor::client::hybrid::ActorHybridClientImpl;
use crate::actor::client::message::{ActorMessageClient, ActorMessageClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::hybrid::ActorHybridServerHandler;
use crate::common::{HybridHandler, Res, StateHandler};

/// Actor implementation to handle messages and requests as well
pub struct HybridActor<ME, MR, R>
where
    ME: Send,
    MR: Send,
    R: Send,
{
    server: ActorServer<ME, MR, R>,
}

impl<ME, MR, R> HybridActor<ME, MR, R>
where
    ME: 'static + Send + Debug,
    MR: 'static + Send + Debug,
    R: 'static + Send + Debug,
{
    /// Create new instance with a message or request handler implementation
    pub(crate) fn new(
        name: String,
        handler: Box<dyn HybridHandler<Message = ME, Request = MR, Reply = R>>,
        state_handler: Box<dyn StateHandler>,
        receive_buffer_size: usize,
    ) -> Self {
        info!(
            "`{}` actor has started with buffer size `{}`",
            name.as_str(),
            receive_buffer_size
        );
        HybridActor {
            server: ActorServer::new(
                name,
                ActorHybridServerHandler::new(handler),
                state_handler,
                receive_buffer_size,
            ),
        }
    }

    /// Return a hybrid client of the actor
    pub fn client(&self) -> Box<dyn ActorHybridClient<ME, MR, R>> {
        debug!("`{}` actor's client has been created", self.server.name());
        Box::new(ActorHybridClientImpl::new(
            self.server.name(),
            self.server.sender(),
        ))
    }

    /// Return a message client of the actor
    pub fn message_client(&self) -> Box<dyn ActorMessageClient<Message = ME> + Send + Sync> {
        debug!("`{}` actor's client has been created", self.server.name());
        Box::new(ActorMessageClientImpl::new(
            self.server.name(),
            self.server.sender(),
        ))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        let name = self.server.name();
        let ret = self.server.stop().await;
        info!("`{}` actor has stopped with result: {:?}", name, ret);
        ret
    }
}
