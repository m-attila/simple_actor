use crate::actor::client::request::{ActorRequestClient, ActorRequestClientImpl};
use crate::actor::server::actor_server::ActorServer;
use crate::actor::server::handler::request::ActorRequestServerHandler;
use crate::common::{RequestHandler, Res, StateHandler};
use log::{debug, info};
use std::fmt::Debug;

/// Actor implementation which can handle synchronous requests
pub struct RequestActor<MR, R>
where
    MR: Send,
    R: Send,
{
    server: ActorServer<(), MR, R>,
}

impl<MR, R> RequestActor<MR, R>
where
    MR: 'static + Send + Debug,
    R: 'static + Send + Debug,
{
    /// Create new instance
    pub(crate) fn new(
        name: String,
        req_handler: Box<dyn RequestHandler<Request = MR, Reply = R>>,
        state_handler: Box<dyn StateHandler>,
        receive_buffer_size: usize,
    ) -> Self {
        info!(
            "`{}` actor has started with buffer size {}",
            name.as_str(),
            receive_buffer_size
        );
        RequestActor {
            server: ActorServer::new(
                name,
                ActorRequestServerHandler::new(req_handler),
                state_handler,
                receive_buffer_size,
            ),
        }
    }

    /// Return a  client of the actor
    pub fn client(&self) -> Box<dyn ActorRequestClient<Request = MR, Reply = R>> {
        debug!("`{}` actor's client has been created", self.server.name());
        Box::new(ActorRequestClientImpl::new(
            self.server.name(),
            self.server.sender(),
        ))
    }

    /// Stop the actor
    pub async fn stop(self) -> Res<()> {
        let name = self.server.name();
        let ret = self.server.stop().await;
        info!("`{}` actor has stopped with result: `{:?}`", name, ret);
        ret
    }
}
