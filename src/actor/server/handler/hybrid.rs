use async_trait::async_trait;

use crate::actor::server::common::ActorServerHandler;
use crate::common::{Command, HybridHandler, Res, SimpleActorError};

/// Message and request handler implementation for hybrid actors
pub(in crate) struct ActorHybridServerHandler<ME: Send, MR: Send, R: Send>
{
    handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>,
}

impl<ME: Send, MR: Send, R: Send> ActorHybridServerHandler<ME, MR, R> {
    /// Creates and wraps in HybridHandler into the implementation
    pub(in crate) fn new(handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>) -> Self {
        ActorHybridServerHandler { handler }
    }
}

#[async_trait]
impl<ME: Send, MR: Send, R: Send> ActorServerHandler for ActorHybridServerHandler<ME, MR, R> {
    type Message = ME;
    type Request = MR;
    type Reply = R;

    async fn process(&mut self, command: Command<Self::Message, Self::Request, Self::Reply>) -> Res<()> {
        match command {
            Command::Request(request, reply_to) => {
                let res = self.handler.process_request(request).await;
                if reply_to.send(res).is_err() {
                    Err(SimpleActorError::Send.into())
                } else { Ok(()) }
            }
            Command::Message(message) =>
                self.handler.process_message(message).await,
            _ => Err(SimpleActorError::UnexpectedCommand.into())
        }
    }
}