use async_trait::async_trait;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::common::{ActorError, Command, RequestHandler, Res, SimpleActorError};

/// Result type of the [`ActorServerHandler::process`] function
pub(crate) type ProcessResult<ME, MR, R> = Res<Option<JoinHandle<Option<Command<ME, MR, R>>>>>;

/// Helps to build [`ProcessResult`] value
pub(crate) struct ProcessResultBuilder<ME, MR, R>(ProcessResult<ME, MR, R>)
    where ME: Send, MR: Send, R: Send;

impl<ME: Send, MR: Send, R: Send> ProcessResultBuilder<ME, MR, R> {
    /// Builds [`ProcessResult`] when the message was processed successfully
    pub(crate) fn message_processed(result: Res<()>) -> Self {
        Self(result.map(|_| None))
    }

    /// Builds [`ProcessResult`] when the message was processed by error
    pub(crate) fn message_processed_with_error(error: ActorError) -> Self {
        Self(Err(error))
    }

    /// Builds [`ProcessResult`] when synchronous request was processed successfully
    pub(crate) fn request_processed() -> Self {
        Self(Ok(None))
    }

    /// Builds [`ProcessResult`] when synchronous request's reply was unable to send to client
    pub(crate) fn request_unable_to_send_reply(handler: &dyn RequestHandler<Request=MR, Reply=R>,
                                               reply: Res<R>) -> Self {
        handler.reply_error(reply);
        Self(Ok(None))
    }

    /// Build [`ProcessResult`] when request is not valid on the context
    pub(crate) fn request_bad() -> Self {
        Self(Err(SimpleActorError::UnexpectedCommand.into()))
    }

    /// Returns process result
    pub(crate) fn result(self) -> ProcessResult<ME, MR, R> {
        self.0
    }
}

/// Result type of the [`ActorAsyncRequestServerHandler::process`] function
pub(crate) type AsyncProcessResult<ME, MR, R> = Option<Command<ME, MR, R>>;

/// Helps to build [`ProcessResult`] value
pub(crate) struct AsyncProcessResultBuilder<ME, MR, R>(AsyncProcessResult<ME, MR, R>)
    where ME: Send, MR: Send, R: Send;

impl<ME: Send, MR: Send, R: Send> AsyncProcessResultBuilder<ME, MR, R> {
    /// Builds [`ProcessResult`] when asynchronous request was executed and its result was transformed to other one
    pub(crate) fn request_transformed_to_async_request(new_request: MR, reply_to: oneshot::Sender<Res<R>>) -> AsyncProcessResultBuilder<ME, MR, R>{
        Self(Some(Command::<ME, MR, R>::Request(new_request, reply_to)))
    }

    /// Builds [`ProcessResult`] when asynchronous request transforming was failed and an error was sent to the requester
    pub(crate) fn request_transformed_to_async_error_was_sent() -> AsyncProcessResultBuilder<ME, MR, R>{
        Self(None)
    }

    /// Builds [`ProcessResult`] when asynchronous request's reply was unable to send to the actor
    pub(crate) fn request_transformed_async_send_error(reply: Res<R>, error: ActorError) -> AsyncProcessResultBuilder<ME, MR, R>{
        Self(Some(Command::<ME, MR, R>::RequestReplyError(reply, error)))
    }

    /// Returns process result
    pub(crate) fn result(self) -> AsyncProcessResult<ME, MR, R> {
        self.0
    }
}

/// This trait should be implemented to handle receiver actor server commands
#[async_trait]
pub(crate) trait ActorServerHandler: Send {
    /// Type of the message
    type Message: Send;
    /// Type of the request
    type Request: Send;
    /// Type of the reply
    type Reply: Send;

    /// Process actor command and return with the result of the processing
    async fn process(&mut self,
                     _message: Command<Self::Message, Self::Request, Self::Reply>) -> ProcessResult<Self::Message, Self::Request, Self::Reply>;
}
