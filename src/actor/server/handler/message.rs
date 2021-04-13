use async_trait::async_trait;

use crate::actor::server::common::ActorServerHandler;
use crate::common::{Command, MessageHandler, Res, SimpleActorError};

/// Message handler implementation for message actors
pub(crate) struct ActorMessageServerHandler<ME: Send> {
    handler: Box<dyn MessageHandler<Message=ME>>
}

impl<ME: Send> ActorMessageServerHandler<ME> {
    /// Creates new instance to wrap in MessageHandler into the implementation
    pub fn new(handler: Box<dyn MessageHandler<Message=ME>>) -> Self {
        ActorMessageServerHandler { handler }
    }
}

#[async_trait]
impl<ME: Send> ActorServerHandler for ActorMessageServerHandler<ME> {
    type Message = ME;
    type Request = ();
    type Reply = ();

    async fn process(&mut self, command: Command<Self::Message, (), ()>) -> Res<()> {
        if let Command::Message(message) = command {
            // It can accept only messages
            self.handler.process_message(message).await
        } else {
            Err(SimpleActorError::UnexpectedCommand.into())
        }
    }
}