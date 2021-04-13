use crate::actor::client::hybrid::ActorHybridClient;
use crate::actor::client::hybrid::ActorHybridClientImpl;
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::hybrid::ActorHybridServerHandler;
use crate::common::{HybridHandler, Res, StateHandler};

/// Actor implementation that can handle messages and requests as well
pub struct HybridActor<ME, MR, R>
    where ME: Send,
          MR: Send,
          R: Send {
    server: ActorServer<ME, MR, R>
}

impl<ME, MR, R> HybridActor<ME, MR, R>
    where ME: 'static + Send,
          MR: 'static + Send,
          R: 'static + Send
{
    /// Creates new instance with given message/request handler implementation
    pub(crate) fn new(handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>,
                      state_handler: Box<dyn StateHandler>,
                      receive_buffer_size: usize,
    ) -> Self {
        HybridActor {
            server: ActorServer::new(
                Box::new(ActorHybridServerHandler::new(handler)),
                state_handler,
                receive_buffer_size,
            )
        }
    }

    /// Returns client for actor
    pub fn client(&self) -> Box<dyn ActorHybridClient<ME, MR, R>> {
        Box::new(ActorHybridClientImpl::new(self.server.sender()))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        self.server.stop().await
    }
}
