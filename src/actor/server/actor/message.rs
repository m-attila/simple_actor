use log::{debug, info};

use crate::actor::client::message::{ActorMessageClient, ActorMessageClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::message::ActorMessageServerHandler;
use crate::common::{MessageHandler, Res, StateHandler};
use std::fmt::Debug;

/// Actor implementation which can handle asynchronous messagges
pub struct MessageActor<ME>
    where ME: Send {
    server: ActorServer<ME, (), ()>
}

impl<ME> MessageActor<ME>
    where ME: 'static + Send + Debug,
{
    /// Creates new instance
    pub(crate) fn new(name: String,
                      msg_handler: Box<dyn MessageHandler<Message=ME>>,
                      state_handler: Box<dyn StateHandler>,
                      receive_buffer_size: usize,
    ) -> Self {
        info!("`{}` actor was started with buffer size `{}`", name.as_str(), receive_buffer_size);
        MessageActor {
            server: ActorServer::new(
                name,
                Box::new(ActorMessageServerHandler::new(msg_handler)),
                state_handler,
                receive_buffer_size,
            )
        }
    }

    /// Returns client for actor
    pub fn client(&self) -> Box<dyn ActorMessageClient<Message=ME> + Send + Sync> {
        debug!("`{}` actor's client was created", self.server.name());
        Box::new(ActorMessageClientImpl::new(self.server.name(),self.server.sender()))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        let name =self.server.name();
        let ret=self.server.stop().await;
        info!("`{}` actor was stopped with result: `{:?}`", name, ret);
        ret
    }
}