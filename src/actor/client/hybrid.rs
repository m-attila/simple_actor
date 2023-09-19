use std::fmt::Debug;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::actor::client::message::{ActorMessageClient, ActorMessageClientImpl};
use crate::actor::client::request::{ActorRequestClient, ActorRequestClientImpl};
use crate::common::{Command, Res};

/// Client specification which can send asynchronous requests and synchronous messages to the actor
#[async_trait]
pub trait ActorHybridClient<ME, MR, R>:
    ActorMessageClient<Message = ME> + ActorRequestClient<Request = MR, Reply = R> + Send + Sync
{
}

/// Boxe message and request client traits
pub(crate) struct ActorHybridClientImpl<ME, MR, R>
where
    ME: Send,
    MR: Send,
    R: Send,
{
    message_client: Box<dyn ActorMessageClient<Message = ME> + Sync>,
    request_client: Box<dyn ActorRequestClient<Request = MR, Reply = R> + Sync>,
}

impl<ME, MR, R> ActorHybridClientImpl<ME, MR, R>
where
    ME: 'static + Send + Debug,
    MR: 'static + Send + Debug,
    R: 'static + Send + Debug,
{
    /// Create new hybrid actor client
    pub(crate) fn new(name: String, sender: mpsc::Sender<Command<ME, MR, R>>) -> Self {
        ActorHybridClientImpl {
            message_client: Box::new(ActorMessageClientImpl::<ME, MR, R>::new(
                name.clone(),
                sender.clone(),
            )),
            request_client: Box::new(ActorRequestClientImpl::<MR, R, ME>::new(name, sender)),
        }
    }
}

#[async_trait]
impl<ME: Send, MR: Send, R: Send> ActorMessageClient for ActorHybridClientImpl<ME, MR, R> {
    type Message = ME;

    async fn message(&self, m: ME) -> Res<()> {
        self.message_client.message(m).await
    }

    fn name(&self) -> String {
        self.message_client.name()
    }
}

#[async_trait]
impl<ME: Send, MR: Send, R: Send> ActorRequestClient for ActorHybridClientImpl<ME, MR, R> {
    type Request = MR;
    type Reply = R;

    async fn request(&self, m: MR) -> Res<R> {
        self.request_client.request(m).await
    }
}

impl<ME: Send, MR: Send, R: Send> ActorHybridClient<ME, MR, R>
    for ActorHybridClientImpl<ME, MR, R>
{
}
