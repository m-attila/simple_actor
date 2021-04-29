use async_trait::async_trait;
use log::{trace, error};
use tokio::sync::mpsc;

use crate::common::{Command, Res, SimpleActorError};
use std::fmt::Debug;

/// Client which can send synchronous message to the actor
#[async_trait]
pub trait ActorMessageClient: Send + Sync{
    /// Type of message
    type Message: Send;
    /// Sends message to the actor
    async fn message(&self, _message: Self::Message) -> Res<()>;

    /// Returns the name of the the actor what belongs to this client
    fn name(&self) -> String;
}

/// Client implementation for message sending
pub(crate) struct ActorMessageClientImpl<ME, MR = (), R = ()>
    where ME: Send,
          MR: Send,
          R: Send {
    name: String,
    sender: mpsc::Sender<Command<ME, MR, R>>,
}

impl<ME, MR, R> ActorMessageClientImpl<ME, MR, R>
    where ME: Send,
          MR: Send,
          R: Send {
    /// Creates new client implementation
    pub(crate) fn new(name: String, sender: mpsc::Sender<Command<ME, MR, R>>) -> Self {
        ActorMessageClientImpl { name, sender }
    }
}

#[async_trait]
impl<ME, MR, R> ActorMessageClient for ActorMessageClientImpl<ME, MR, R>
    where ME: Send + Debug,
          MR: Send + Debug,
          R: Send + Debug {
    type Message = ME;

    async fn message(&self, m: ME) -> Res<()> {
        trace!("`{}` actor client will send message: `{:?}`", self.name(), &m);
        self.sender.send(Command::Message(m))
            .await
            .map_err(|e| {
                error!("`{}` actor client unable to send message: `{:?}`", self.name(), e);
                SimpleActorError::Send.into()
            })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

