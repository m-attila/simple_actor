use crate::actor::client::request::{ActorRequestClient, ActorRequestClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::request::ActorRequestServerHandler;
use crate::common::{RequestHandler, Res, StateHandler};

/// Actor implementation which can handle asynchronous requests
pub struct RequestActor<MR, R>
    where MR: Send,
          R: Send {
    server: ActorServer<(), MR, R>
}

impl<MR, R> RequestActor<MR, R>
    where MR: 'static + Send,
          R: 'static + Send
{
    /// Creates new instance
    pub(crate) fn new(req_handler: Box<dyn RequestHandler<Request=MR, Reply=R>>,
                      state_handler: Box<dyn StateHandler>,
                      receive_buffer_size: usize,
    ) -> Self {
        RequestActor {
            server: ActorServer::new(
                Box::new(ActorRequestServerHandler::new(req_handler)),
                state_handler,
                receive_buffer_size,
            )
        }
    }

    /// Returns client for actor
    pub fn client(&self) -> Box<dyn ActorRequestClient<Request=MR, Reply=R>> {
        Box::new(ActorRequestClientImpl::new(self.server.sender()))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        self.server.stop().await
    }
}