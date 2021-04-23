use async_trait::async_trait;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::common::{ActorError, Command, RequestHandler, Res, SimpleActorError};

/// Result type of the [`ActorServerHandler::process`] function
pub(crate) type ProcessResult<ME, MR, R> = Res<Option<JoinHandle<Option<Command<ME, MR, R>>>>>;

/// Helps to build [`ProcessResult`] value
pub(crate) struct ProcessResultBuilder;

impl ProcessResultBuilder {
    /// Builds [`ProcessResult`] when message was processed with any result
    pub(crate) fn message_processed<ME, MR, R>(result: Res<()>) -> ProcessResult<ME, MR, R>
        where ME: Send, MR: Send, R: Send {
        result.map(|_| None)
    }

    /// Builds [`ProcessResult`] when message was processed by internal error
    pub(crate) fn message_processed_with_error<ME, MR, R>(error: ActorError) -> ProcessResult<ME, MR, R>
        where ME: Send, MR: Send, R: Send {
        Err(error)
    }

    /// Builds [`ProcessResult`] when synchronous request was processed successfully
    pub(crate) fn request_processed<ME, MR, R>() -> ProcessResult<ME, MR, R>
        where ME: Send, MR: Send, R: Send {
        Ok(None)
    }

    /// Builds [`ProcessResult`] when synchronous request's reply was not send to client
    pub(crate) fn request_unable_to_send_reply<ME, MR, R>(handler: &dyn RequestHandler<Request=MR, Reply=R>,
                                                          reply: Res<R>) -> ProcessResult<ME, MR, R>
        where ME: Send, MR: Send, R: Send {
        handler.reply_error(reply);
        Ok(None)
    }

    /// Build [`ProcessResult`] when request is not valid on the context
    pub(crate) fn request_bad<ME, MR, R>() -> ProcessResult<ME, MR, R>
        where ME: Send, MR: Send, R: Send {
        Err(SimpleActorError::UnexpectedCommand.into())
    }

    /// Builds [`ProcessResult`] when asynchronous request was transformed the synchronous one
    pub(crate) fn request_transformed_to_async_request<ME, MR, R>(new_request: MR, reply_to: oneshot::Sender<Res<R>>) -> Option<Command<ME, MR, R>>
        where ME: Send, MR: Send, R: Send {
        Some(Command::<ME, MR, R>::Request(new_request, reply_to))
    }

    /// Builds [`ProcessResult`] when asynchronous request transforming was failed and error was sent to the requester
    pub(crate) fn request_transformed_to_async_error_was_sent<ME, MR, R>() -> Option<Command<ME, MR, R>>
        where ME: Send, MR: Send, R: Send {
        None
    }

    /// Builds [`ProcessResult`] when asynchronous request reply was failed internally
    pub(crate) fn request_transformed_async_send_error<ME, MR, R>(reply: Res<R>, error: ActorError) -> Option<Command<ME, MR, R>>
        where ME: Send, MR: Send, R: Send {
        Some(Command::<ME, MR, R>::RequestReplyError(reply, error))
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
