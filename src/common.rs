//! Common types and structs in simple_actor.
use std::any::Any;
use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use log::error;
use tokio::sync::oneshot;

/// Generic error type
pub type ActorError = Box<dyn std::error::Error + Send + Sync>;
/// Simplified result type
pub type Res<T> = Result<T, ActorError>;

/// simple_actor errors
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SimpleActorError {
    /// Receive error on the channel
    Receive,
    /// Send error on the channel
    Send,
    /// Unexpected command in the execution context
    UnexpectedCommand,
}

impl std::fmt::Display for SimpleActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleActorError::Receive => write!(f, "Receive error"),
            SimpleActorError::Send => write!(f, "Send error"),
            SimpleActorError::UnexpectedCommand => write!(f, "Unexpected command"),
        }
    }
}

impl std::error::Error for SimpleActorError {}

impl TryFrom<&ActorError> for SimpleActorError {
    type Error = &'static str;

    fn try_from(value: &ActorError) -> Result<Self, Self::Error> {
        match value.downcast_ref::<SimpleActorError>() {
            None => Err("Not a SimpleActorError type"),
            Some(res) => Ok(*res)
        }
    }
}

unsafe impl Send for SimpleActorError {}

unsafe impl Sync for SimpleActorError {}

/// Actor commands
#[derive(Debug)]
pub(crate) enum Command<ME: Send, MR: Send, R: Send> {
    /// Asynchronous message without waiting for any response
    Message(ME),
    /// Synchronous request with response
    Request(MR, oneshot::Sender<Res<R>>),
    /// Stop the actor
    Stop,
    /// Unable to send reply for processed asynchronous request
    RequestReplyError(Res<R>, ActorError),
}


/// This trait should be implemented to process actor's asynchronous messages.
#[async_trait]
pub trait MessageHandler: Send {
    /// Type of the actor's message
    type Message: Send;

    /// Message processor.
    /// This method processes `message` argument and when the processing is success, acknowledges it with [`enum@Result::Ok`].
    /// If method returns with [`enum@Result::Err`] with [`type@ActorError`] the actor will stop immediately.
    /// In this case, the actor's `stop` method will be returned with the error of the `process_message` function.
    async fn process_message(&mut self, _message: Self::Message) -> Res<()>;
}

/// Classified request. Indicates how can be process the received request.
pub enum RequestExecution<R> {
    /// The request will be processed synchronously. Until the processing finshed,
    /// the actor does not step to the next message or request
    Sync(R),
    /// The request will be processed asynchronously. During its processing, the actor
    /// start to deal with the next message or request.
    Async(R),
    /// Such as the [`Async`](enum@RequestExecution::Async) execution, but it is a blocking processing, the actor start to deal
    /// with it in a new thread. See [`spawn_blocking`](fn@tokio::task::spawn_blocking) method.
    Blocking(R),
}

/// This trait should be implemented to process actor's synchronous requests.
#[async_trait]
pub trait RequestHandler: Send {
    /// Type of request
    type Request: Send;
    /// Type of reply
    type Reply: Send;

    /// Classify the request by execution mode
    async fn classify_request(&mut self, request: Self::Request) -> RequestExecution<Self::Request> {
        RequestExecution::Sync(request)
    }

    /// Request processor.
    /// Process `request` argument and returns with processing reply. The reply could be
    /// `Ok(:Reply)` or `Err(:ActorError)` which will be returned into the actor's client.
    async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply>;

    /// Returns a function which executes the blocking operation. This computation will not
    /// block any other receiving and processing mechanism. When the computation has finished successfully,
    /// this function returns a new request which wraps in the computation's result. The actor process this new
    /// request serially as any other ones. With this serialization method, the computation's result
    /// could be affect for the actor's state.
    fn get_blocking_transformation(&self) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        error!("Please implement get_blocking_transformation(...) method in RequestHandler implementation");
        unimplemented!()
    }

    /// Same as the [`get_blocking_transformation`](fn@self::RequestHandler::get_blocking_transformation) method, but the returned
    /// function will be executed by async operation in a spanned task which is started with
    /// [`spawn`](fn@tokio::spawn)
    fn get_async_transformation(&self) -> Box<dyn Fn(Self::Request) -> Pin<Box<dyn Future<Output=Res<Self::Request>> + Send>> + Send + Sync> {
        error!("Please implement get_async_transformation(...) method in RequestHandler implementation");
        unimplemented!()
    }

    /// In this method can be handle those errors, which occurs when the send of reply was failed.
    fn reply_error(&self, _result: Res<Self::Reply>) {
        error!("Unable to send reply for request. Please reimplement reply_error(...) method in RequestHandler implementation");
    }
}

/// This trait should be implemented to process actor's synchronous messages and asynchronous requests as well.
#[async_trait]
pub trait HybridHandler: Any + Send + MessageHandler + RequestHandler {}

/// This trait should be implemented to handle actor initialization and terminate events.
pub trait StateHandler: Send {
    /// Initialization event.
    /// If this method returns with `Ok(())` the initialization was success otherwise it was failed.
    /// If the initialization was failed, the actor stops immediately.
    fn init(&mut self, name: String) -> Res<()>;

    /// Actor terminate event.
    /// This method receives the reason of termination in `reason` argument.
    fn terminate(&mut self, name: String, _reason: &Res<()>);
}

/// This enum helps to identity the boxed dynamic error's type.
pub enum ActorErrorHandler<'a> {
    /// The boxed error does not identified
    Unprocessed(&'a Box<dyn std::error::Error + Send + Sync>),
    /// The boxed error identified and processes
    Processed,
}

impl<'a> ActorErrorHandler<'a> {
    /// Creates new instance for dynamic boxed instance
    pub fn new(error: &'a Box<dyn std::error::Error + Send + Sync>) -> Self {
        ActorErrorHandler::Unprocessed(error)
    }

    /// If the boxed error can be identified by given `T` type, then the `func` function will process it.
    pub fn and_then<T: 'static + std::error::Error + Send + Sync>(&self, func: &dyn Fn(&T)) -> &Self {
        match &self {
            Self::Processed => self,
            Self::Unprocessed(dynamic) => {
                if let Some(typed) = dynamic.downcast_ref::<T>() {
                    func(typed);
                    return &Self::Processed;
                }
                self
            }
        }
    }

    /// Process the boxed error if it could not be identified with `and_then` methods previously.
    /// The `func` function could handle this case.
    pub fn or_else(&self, func: &dyn Fn(&Box<dyn std::error::Error + Send + Sync>)) {
        if let Self::Unprocessed(dynamic) = &self {
            func(dynamic)
        }
    }
}
