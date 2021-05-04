use std::any::Any;

use async_trait::async_trait;

use crate::actor::server::common::{ActorServerHandler, ProcessResult, ProcessResultBuilder};
use crate::actor::server::handler::request::ActorAsyncRequestServerHandler;
use crate::common::{Command, HybridHandler, RequestHandler, SimpleActorError};

/// Message and request handler implementation for hybrid actors
pub(in crate) struct ActorHybridServerHandler<ME: Send, MR: Send, R: Send>
{
    handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>,
}

impl<ME: 'static, MR: 'static, R: 'static> ActorHybridServerHandler<ME, MR, R>
    where ME: Send, MR: Send, R: Send {
    /// Creates and wraps in HybridHandler into the implementation
    pub(in crate) fn new(handler: Box<dyn HybridHandler<Message=ME, Request=MR, Reply=R>>) -> Self {
        ActorHybridServerHandler { handler }
    }

    /// Converts HybridHandler to RequestHandler
    fn as_request_handler_ref(&self) -> &dyn RequestHandler<Request=MR, Reply=R> {
        let x_any = &self.handler as &dyn Any;
        x_any.downcast_ref::<Box<dyn RequestHandler<Request=MR, Reply=R>>>().unwrap().as_ref()
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
            Command::Request(request, reply_to) => {
                if !self.handler.is_heavy(&request) {
                    let res = self.handler.process_request(request).await;
                    if let Err(e) = reply_to.send(res) {
                        let ref_handler = self.as_request_handler_ref();
                        ProcessResultBuilder::request_unable_to_send_reply(ref_handler, e).result()
                    } else {
                        ProcessResultBuilder::request_processed().result()
                    }
                } else {
                    let transformation = self.handler.get_heavy_transformation();
                    ActorAsyncRequestServerHandler::process::<Self::Message, Self::Request, Self::Reply>(request, reply_to, transformation)
                }
            }
            Command::Message(message) =>
                ProcessResultBuilder::message_processed(self.handler.process_message(message).await).result(),
            Command::RequestReplyError(res, _error) => {
                self.handler.reply_error(res);
                ProcessResultBuilder::request_processed().result()
            }
            _ => ProcessResultBuilder::message_processed_with_error(SimpleActorError::UnexpectedCommand.into()).result()
        }
    }
}