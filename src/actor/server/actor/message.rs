use crate::actor::client::message::{ActorMessageClient, ActorMessageClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::message::ActorMessageServerHandler;
use crate::common::{MessageHandler, Res, StateHandler};

/// Actor implementation which can handle asynchronous messagges
pub struct MessageActor<ME>
    where ME: Send {
    server: ActorServer<ME, (), ()>
}

impl<ME> MessageActor<ME>
    where ME: 'static + Send,
{
    /// Creates new instance
    pub(crate) fn new(msg_handler: Box<dyn MessageHandler<Message=ME>>,
                      state_handler: Box<dyn StateHandler>,
                      receive_buffer_size: usize,
    ) -> Self {
        MessageActor {
            server: ActorServer::new(
                Box::new(ActorMessageServerHandler::new(msg_handler)),
                state_handler,
                receive_buffer_size,
            )
        }
    }

    /// Returns client for actor
    pub fn client(&self) -> Box<dyn ActorMessageClient<Message=ME> + Send + Sync> {
        Box::new(ActorMessageClientImpl::new(self.server.sender()))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        self.server.stop().await
    }
}