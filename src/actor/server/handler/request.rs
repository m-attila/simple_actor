use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::actor::server::common::{
    ActorServerHandler, AsyncProcessResult, AsyncProcessResultBuilder, ProcessResult,
    ProcessResultBuilder,
};
use crate::common::{Command, RequestExecution, RequestHandler, Res, SimpleActorError};

pub(crate) struct RequestProcessor();

impl RequestProcessor {
    /// This function processes requests
    pub(crate) async fn process<ME, MR, R>(
        handler: &mut dyn RequestHandler<Request = MR, Reply = R>,
        command: Command<ME, MR, R>,
    ) -> ProcessResult<ME, MR, R>
    where
        ME: Send + 'static,
        MR: Send + 'static,
        R: Send + 'static,
    {
        match command {
            Command::Request(request, reply_to) => match handler.classify_request(request).await {
                RequestExecution::Sync(request) => {
                    let res = handler.process_request(request).await;
                    if let Err(e) = reply_to.send(res) {
                        ProcessResultBuilder::request_unable_to_send_reply(handler, e).result()
                    } else {
                        ProcessResultBuilder::request_processed().result()
                    }
                }
                RequestExecution::Async(request) => {
                    let transformation = handler.get_async_transformation();
                    ActorAsyncRequestServerHandler::process_async::<ME, MR, R>(
                        request,
                        reply_to,
                        transformation,
                    )
                }
                RequestExecution::Blocking(request) => {
                    let transformation = handler.get_blocking_transformation();
                    ActorAsyncRequestServerHandler::process_blocking::<ME, MR, R>(
                        request,
                        reply_to,
                        transformation,
                    )
                }
            },
            Command::RequestReplyError(res, _error) => {
                handler.reply_error(res);
                ProcessResultBuilder::request_processed().result()
            }
            _ => ProcessResultBuilder::request_bad().result(),
        }
    }
}

/// Request handler implementation for request handling actors
pub(crate) struct ActorRequestServerHandler<MR: Send, R: Send>(
    Box<dyn RequestHandler<Request = MR, Reply = R>>,
);

impl<MR, R> ActorRequestServerHandler<MR, R>
where
    MR: Send + 'static,
    R: Send + 'static,
{
    /// Wrap `RequestHandler` into this implementation
    pub(crate) fn new(handler: Box<dyn RequestHandler<Request = MR, Reply = R>>) -> Self {
        ActorRequestServerHandler(handler)
    }
}

#[async_trait]
impl<MR, R> ActorServerHandler for ActorRequestServerHandler<MR, R>
where
    MR: 'static + Send,
    R: 'static + Send,
{
    type Message = ();
    type Request = MR;
    type Reply = R;

    async fn process(
        &mut self,
        command: Command<(), Self::Request, Self::Reply>,
    ) -> ProcessResult<Self::Message, Self::Request, Self::Reply> {
        RequestProcessor::process::<(), MR, R>(self.0.as_mut(), command).await
    }
}

/// Handle asynchronous processing requests
struct ActorAsyncRequestServerHandler;

impl ActorAsyncRequestServerHandler {
    /// Process a blocking request. Useful to calculate heavy computations
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn process_blocking<ME, MR, R>(
        request: MR,
        reply_to: oneshot::Sender<Res<R>>,
        transformation: Box<dyn Fn(MR) -> Res<MR> + Send>,
    ) -> ProcessResult<ME, MR, R>
    where
        ME: Send + 'static,
        MR: Send + 'static,
        R: Send + 'static,
    {
        let handle = tokio::task::spawn_blocking(move || {
            // execute heavy computation, and receive its result
            ActorAsyncRequestServerHandler::transformation_result::<ME, MR, R>(
                transformation(request),
                reply_to,
            )
        });
        Ok(Some(handle))
    }

    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::type_complexity)]
    /// Process an asynchronous request within a separate task
    pub(crate) fn process_async<ME, MR, R>(
        request: MR,
        reply_to: oneshot::Sender<Res<R>>,
        transformation: Box<
            dyn Fn(MR) -> Pin<Box<dyn Future<Output = Res<MR>> + Send>> + Send + Sync,
        >,
    ) -> ProcessResult<ME, MR, R>
    where
        ME: Send + 'static,
        MR: Send + 'static,
        R: Send + 'static,
    {
        let handle = tokio::spawn(async move {
            let res = transformation(request).await;
            // execute heavy computation, and receive its result
            ActorAsyncRequestServerHandler::transformation_result::<ME, MR, R>(res, reply_to)
        });
        Ok(Some(handle))
    }

    fn transformation_result<ME, MR, R>(
        res: Res<MR>,
        reply_to: oneshot::Sender<Res<R>>,
    ) -> AsyncProcessResult<ME, MR, R>
    where
        ME: Send + 'static,
        MR: Send + 'static,
        R: Send + 'static,
    {
        match res {
            // transform computing result into a new request
            Ok(req) => {
                AsyncProcessResultBuilder::request_transformed_to_async_request(req, reply_to)
                    .result()
            }
            Err(err) => {
                // Send error result back to the client immediately
                if let Err(res) = reply_to.send(Err(err)) {
                    AsyncProcessResultBuilder::request_transformed_async_send_error(
                        res,
                        SimpleActorError::Send.into(),
                    )
                    .result()
                } else {
                    AsyncProcessResultBuilder::request_transformed_to_async_error_was_sent()
                        .result()
                }
            }
        }
    }
}
