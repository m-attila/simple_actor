use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::actor::server::common::{ActorServerHandler, ProcessResult, ProcessResultBuilder, AsyncProcessResultBuilder};
use crate::common::{Command, RequestHandler, Res, SimpleActorError};

/// Request handler implementation for request actors
pub(in crate) struct ActorRequestServerHandler<MR: Send, R: Send> {
    handler: Box<dyn RequestHandler<Request=MR, Reply=R>>
}

impl<MR: Send, R: Send> ActorRequestServerHandler<MR, R> {
    // Creates new instance to wrap RequestHandler into the implementation
    pub(crate) fn new(handler: Box<dyn RequestHandler<Request=MR, Reply=R>>) -> Self {
        ActorRequestServerHandler { handler }
    }
}

#[async_trait]
impl<MR, R> ActorServerHandler for ActorRequestServerHandler<MR, R>
    where MR: 'static + Send, R: 'static + Send {
    type Message = ();
    type Request = MR;
    type Reply = R;

    async fn process(&mut self, command: Command<(), Self::Request, Self::Reply>) -> ProcessResult<Self::Message, Self::Request, Self::Reply> {
        match command {
            Command::Request(request, reply_to) => {
                if !self.handler.is_heavy(&request) {
                    let res = self.handler.process_request(request).await;
                    if let Err(e) = reply_to.send(res) {
                        ProcessResultBuilder::request_unable_to_send_reply(self.handler.as_ref(), e).result()
                    } else {
                        ProcessResultBuilder::request_processed().result()
                    }
                } else {
                    let transformation = self.handler.get_heavy_transformation();
                    ActorAsyncRequestServerHandler::process::<Self::Message, Self::Request, Self::Reply>(request, reply_to, transformation)
                }
            }
            Command::RequestReplyError(res, _error) => {
                self.handler.reply_error(res);
                ProcessResultBuilder::request_processed().result()
            }
            _ =>
                ProcessResultBuilder::request_bad().result()
        }
    }
}

/// Handle async processing requests
pub(crate) struct ActorAsyncRequestServerHandler;

impl ActorAsyncRequestServerHandler {
    /// Process async requests
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn process<ME, MR, R>(request: MR, reply_to: oneshot::Sender<Res<R>>, transformation: Box<dyn Fn(MR) -> Res<MR> + Send>) -> ProcessResult<ME, MR, R>
        where ME: Send + 'static,
              MR: Send + 'static,
              R: Send + 'static {
        let handle = tokio::task::spawn_blocking(move || {
            // execute heavy computation, and receive its result
            let res: Res<MR> = transformation(request);
            match res {
                // transform computing result into a new request
                Ok(req) => AsyncProcessResultBuilder::request_transformed_to_async_request(req, reply_to).result(),
                Err(err) => {
                    // Send error result back to the client immediately
                    if let Err(res) = reply_to.send(Err(err)) {
                        AsyncProcessResultBuilder::request_transformed_async_send_error(res, SimpleActorError::Send.into()).result()
                    } else {
                        AsyncProcessResultBuilder::request_transformed_to_async_error_was_sent().result()
                    }
                }
            }
        });
        Ok(Some(handle))
    }
}
