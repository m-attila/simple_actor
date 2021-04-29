use log::{info, debug};
use crate::actor::client::request::{ActorRequestClient, ActorRequestClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::request::ActorRequestServerHandler;
use crate::common::{RequestHandler, Res, StateHandler};
use std::fmt::Debug;

/// Actor implementation which can handle asynchronous requests
pub struct RequestActor<MR, R>
    where MR: Send,
          R: Send {
    server: ActorServer<(), MR, R>
}

impl<MR, R> RequestActor<MR, R>
    where MR: 'static + Send + Debug,
          R: 'static + Send + Debug
{
    /// Creates new instance
    pub(crate) fn new(name: String,
                      req_handler: Box<dyn RequestHandler<Request=MR, Reply=R>>,
                      state_handler: Box<dyn StateHandler>,
                      receive_buffer_size: usize,
    ) -> Self {
        info!("`{}` actor was started with buffer size {}", name.as_str(), receive_buffer_size);
        RequestActor {
            server: ActorServer::new(
                name,
                ActorRequestServerHandler::new(req_handler),
                state_handler,
                receive_buffer_size,
            )
        }
    }

    /// Returns client for actor
    pub fn client(&self) -> Box<dyn ActorRequestClient<Request=MR, Reply=R>> {
        debug!("`{}` actor's client was created", self.server.name());
        Box::new(ActorRequestClientImpl::new(self.server.name(), self.server.sender()))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        let name =self.server.name();
        let ret=self.server.stop().await;
        info!("`{}` actor was stopped with result: `{:?}`", name, ret);
        ret
    }
}