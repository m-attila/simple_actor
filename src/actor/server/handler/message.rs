use async_trait::async_trait;

use crate::actor::server::common::{ActorServerHandler, ProcessResult,ProcessResultBuilder};
use crate::common::{Command, MessageHandler, SimpleActorError};

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

    async fn process(&mut self, command: Command<Self::Message, (), ()>) -> ProcessResult<Self::Message, Self::Request, Self::Reply> {
        if let Command::Message(message) = command {
            ProcessResultBuilder::message_processed(self.handler.process_message(message).await)
        } else {
            ProcessResultBuilder::message_processed_with_error(SimpleActorError::UnexpectedCommand.into())
        }
    }
}