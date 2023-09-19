use async_trait::async_trait;
use log::{error, trace};
use tokio::sync::{mpsc, oneshot};

use crate::common::{Command, Res, SimpleActorError};
use std::fmt::Debug;

/// Client which can send synchronous requests and receive replies from the actor
#[async_trait]
pub trait ActorRequestClient: Send + Sync {
    type Request: Send;
    type Reply: Send;

    /// Send request to actor
    async fn request(&self, m: Self::Request) -> Res<Self::Reply>;
}

/// Client for request sending
pub(crate) struct ActorRequestClientImpl<MR, R, ME = ()>
where
    MR: Send,
    R: Send,
    ME: Send,
{
    name: String,
    sender: mpsc::Sender<Command<ME, MR, R>>,
}

impl<MR, R, ME> ActorRequestClientImpl<MR, R, ME>
where
    MR: Send,
    R: Send,
    ME: Send,
{
    /// Create new client implementation
    pub(crate) fn new(name: String, sender: mpsc::Sender<Command<ME, MR, R>>) -> Self {
        ActorRequestClientImpl { name, sender }
    }

    /// Return the name of the the actor which belongs to this client
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[async_trait]
impl<MR, R, ME> ActorRequestClient for ActorRequestClientImpl<MR, R, ME>
where
    MR: Send + Debug,
    R: Send + Debug,
    ME: Send + Debug,
{
    type Request = MR;
    type Reply = R;

    async fn request(&self, m: MR) -> Res<R> {
        let (send_reply, rec_reply) = oneshot::channel();
        trace!(
            "`{}` actor client will send this request: `{:?}`",
            self.name(),
            &m
        );
        match self.sender.send(Command::Request(m, send_reply)).await {
            Ok(_) => match rec_reply.await {
                Ok(reply) => {
                    trace!(
                        "`{}` actor client reply was received: `{:?}`",
                        self.name(),
                        &reply
                    );
                    reply
                }
                Err(e) => Err({
                    error!(
                        "`{}` actor client unable to receive reply: `{:?}`",
                        self.name(),
                        e
                    );
                    SimpleActorError::Receive.into()
                }),
            },
            Err(e) => Err({
                error!(
                    "`{}` actor client unable to send request: `{:?}`",
                    self.name(),
                    e
                );
                SimpleActorError::Send.into()
            }),
        }
    }
}
