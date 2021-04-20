use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::actor::server::common::{ActorServerHandler, ProcessResult, ProcessResultBuilder};
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
        if let Command::Request(request, reply_to) = command {
            if !self.handler.is_heavy(&request) {
                // It can accept only requests
                let res = self.handler.process_request(request).await;
                if reply_to.send(res).is_err() {
                    // Request processing was failed
                    ProcessResultBuilder::request_processed_with_error(SimpleActorError::Send.into())
                } else { ProcessResultBuilder::request_processed() }
            } else {
                let transformation = self.handler.get_heavy_transformation();
                ActorAsyncRequestServerHandler::process::<Self::Message, Self::Request, Self::Reply>(request, reply_to, transformation)
            }
        } else {
            ProcessResultBuilder::request_processed_with_error(SimpleActorError::UnexpectedCommand.into())
        }
    }
}

/// Handle async requests
pub(crate) struct ActorAsyncRequestServerHandler;

impl ActorAsyncRequestServerHandler {
    /// Process async requests
    pub(crate) fn process<ME, MR, R>(request: MR, reply_to: oneshot::Sender<Res<R>>, transformation: Box<dyn Fn(MR) -> Res<MR> + Send>) -> ProcessResult<ME, MR, R>
        where ME: Send + 'static,
              MR: Send + 'static,
              R: Send + 'static {
        let handle = tokio::task::spawn_blocking( move || {
            // execute heavy computing, and receive its result
            let res: Res<MR> = transformation(request);
            match res {
                // turn computing result into new request
                Ok(req) => ProcessResultBuilder::request_transformed_to_async_request(req, reply_to),
                // Command::<Self::Message, Self::Request, Self::Reply>::Request(req, reply_to),
                Err(err) => {
                    // Send error result back to the client immediate
                    if reply_to.send(Err(err).into()).is_ok() {
                        ProcessResultBuilder::request_transformed_to_async_error_was_sent()
                    } else {
                        // @TODO: how to handle heavy computation errors
                        ProcessResultBuilder::request_transformed_async_send_error(SimpleActorError::Send.into())
                    }
                }
            }
        });
        Ok(Some(handle))
    }
}
