use async_trait::async_trait;

use crate::actor::server::common::{ActorServerHandler, ProcessResult, ProcessResultBuilder};
use crate::actor::server::handler::request::RequestProcessor;
use crate::common::{Command, HybridHandler};

/// Message and request handler implementation for hybrid actors
pub(in crate) struct ActorHybridServerHandler<ME: Send, MR: Send, R: Send>
(
    Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>,
);

unsafe impl<ME: Send, MR: Send, R: Send> Send for ActorHybridServerHandler<ME, MR, R> {}

unsafe impl<ME: Send, MR: Send, R: Send> Sync for ActorHybridServerHandler<ME, MR, R> {}

impl<ME: 'static, MR: 'static, R: 'static> ActorHybridServerHandler<ME, MR, R>
    where ME: Send, MR: Send, R: Send {
    /// Creates and wraps in HybridHandler into the implementation
    pub(in crate) fn new(handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>) -> Self {
        ActorHybridServerHandler(handler)
    }
}

#[async_trait]
impl<ME, MR, R> ActorServerHandler for ActorHybridServerHandler<ME, MR, R>
    where ME: Send + 'static, MR: Send + 'static, R: Send + 'static {
    type Message = ME;
    type Request = MR;
    type Reply = R;

    async fn process(&mut self, command: Command<Self::Message, Self::Request, Self::Reply>) -> ProcessResult<Self::Message, Self::Request, Self::Reply> {
        match command {
            Command::Message(message) =>
                ProcessResultBuilder::message_processed(self.0.process_message(message).await).result(),
            command @ _ =>
                RequestProcessor::process::<ME, MR, R>(self.0.request_handler_mut(), command).await
        }
    }
}