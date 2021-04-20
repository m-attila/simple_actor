use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::common::{Command, Res, SimpleActorError};

/// Client which can send synchronous requests and receive replies from the actor
#[async_trait]
pub trait ActorRequestClient: Send + Sync{
    type Request: Send;
    type Reply: Send;

    /// Send request to actor
    async fn request(&self, m: Self::Request) -> Res<Self::Reply>;
}

/// Client for request sending
pub(crate) struct ActorRequestClientImpl<MR, R, ME = ()>
    where MR: Send,
          R: Send,
          ME: Send {
    sender: mpsc::Sender<Command<ME, MR, R>>,
}

impl<MR, R, ME> ActorRequestClientImpl<MR, R, ME>
    where MR: Send,
          R: Send,
          ME: Send {
    /// Creates new client implementation
    pub(crate) fn new(sender: mpsc::Sender<Command<ME, MR, R>>) -> Self {
        ActorRequestClientImpl { sender }
    }
}

#[async_trait]
impl<MR, R, ME> ActorRequestClient for ActorRequestClientImpl<MR, R, ME>
    where MR: Send,
          R: Send,
          ME: Send {
    type Request = MR;
    type Reply = R;

    async fn request(&self, m: MR) -> Res<R> {
        let (send_reply, rec_reply) = oneshot::channel();
        match self.sender.send(Command::Request(m, send_reply)).await {
            Ok(_) => {
                match rec_reply.await {
                    Ok(reply) => reply,
                    Err(_) => Err(SimpleActorError::Receive.into())
                }
            }
            Err(_) => Err(SimpleActorError::Send.into())
        }
    }
}
