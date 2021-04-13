use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::common::{Command, Res, SimpleActorError};

/// Client which can send synchronous message to the actor
#[async_trait]
pub trait ActorMessageClient: Send {
    /// Type of message
    type Message: Send;
    /// Sends message to the actor
    async fn message(&self, _message: Self::Message) -> Res<()>;
}

/// Client implementation for message sending
pub(crate) struct ActorMessageClientImpl<ME, MR = (), R = ()>
    where ME: Send,
          MR: Send,
          R: Send {
    sender: mpsc::Sender<Command<ME, MR, R>>,
}

impl<ME, MR, R> ActorMessageClientImpl<ME, MR, R>
    where ME: Send,
          MR: Send,
          R: Send {
    /// Creates new client implementation
    pub(crate) fn new(sender: mpsc::Sender<Command<ME, MR, R>>) -> Self {
        ActorMessageClientImpl { sender }
    }
}

#[async_trait]
impl<ME, MR, R> ActorMessageClient for ActorMessageClientImpl<ME, MR, R>
    where ME: Send,
          MR: Send,
          R: Send {
    type Message = ME;

    async fn message(&self, m: ME) -> Res<()> {
        self.sender.send(Command::Message(m))
            .await
            .map_err(|_| SimpleActorError::Send.into())
    }
}

