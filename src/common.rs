//! Common types and structs for actors
use log::{error};
use std::convert::TryFrom;

use async_trait::async_trait;
use tokio::sync::oneshot;
use std::any::Any;

/// Generic actor error type
pub type ActorError = Box<dyn std::error::Error + Send + Sync>;
/// Simplified result type
pub type Res<T> = Result<T, ActorError>;

/// Internal error type
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SimpleActorError {
    /// Receive error
    Receive,
    /// Send error
    Send,
    /// Unexpected command in context
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
    /// Asynchronous message without response
    Message(ME),
    /// Synchronous message with response
    Request(MR, oneshot::Sender<Res<R>>),
    /// Stop the actor
    Stop,
    /// Unable to send reply for processed heavy computing request
    RequestReplyError(Res<R>, ActorError),
}


/// This trait should be implemented to process actor's synchronous messages.
#[async_trait]
pub trait MessageHandler: Send {
    /// Type of the message
    type Message: Send;

    /// Message processor
    /// Process `message` argument and acknowledge with [`enum@Result::Ok`]  about it.
    /// If method returns with [`enum@Result::Err`] with [`type@ActorError`] the actor will be stopped.
    /// The actor's `stop` method will be returned with the error.
    async fn process_message(&mut self, _message: Self::Message) -> Res<()>;
}

/// This trait should be implemented to process actor's asynchronous requests.
#[async_trait]
pub trait RequestHandler: Send {
    /// Type of request
    type Request: Send;
    /// Type of reply
    type Reply: Send;

    /// Request processor
    /// Process `request` argument and returns with reply. The reply could be
    /// `Ok(:Reply)` or `Err(:ActorError)` that will be received by actor's client.
    async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply>;

    /// Returns if the request requires heavy computing
    fn is_heavy(&self, _request: &Self::Request) -> bool {
        false
    }

    /// Returns handle function which generates reply from request in heavy computing process
    fn get_heavy_transformation(&self) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        error!("Please implement get_heavy_transformation(...) method in RequestHandler implementation");
        unimplemented!()
    }

    /// Unable to send reply for request
    fn reply_error(&self, _result: Res<Self::Reply>){
        error!("Unable to send reply for request. Please reimplement reply_error(...) method in RequestHandler implementation");
    }
}

/// This trait should be implemented to process actor's synchronous messages and asynchronous requests as well.
#[async_trait]
pub trait HybridHandler: Any + Send + MessageHandler + RequestHandler {}

/// This trait should be implemented to handle actor initialization and terminate events.
pub trait StateHandler: Send {
    /// Initialization event
    /// If this method returns with `Ok(())` the initialization was success otherwise it was fail and
    /// actor stops immediately.
    fn init(&mut self, name: String) -> Res<()>;

    /// Actor terminate event
    /// This method receives the reason of termination in `reason` argument.
    fn terminate(&mut self, name: String, _reason: &Res<()>);
}

/// Helps to identify and process boxed dynamic error by custom error type.
pub enum ActorErrorHandler<'a> {
    /// The boxed error does not identified
    Unprocessed(&'a Box<dyn std::error::Error + Send + Sync>),
    /// The boxed error identified and processes
    Processed,
}

impl<'a> ActorErrorHandler<'a> {
    /// Creates new handler instance from dynamic boxed instance
    pub fn new(error: &'a Box<dyn std::error::Error + Send + Sync>) -> Self {
        ActorErrorHandler::Unprocessed(error)
    }

    /// Identity type of dynamic boxed error instance, and process it
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

    /// Process dynamic boxed error if its type is unidentified
    pub fn or_else(&self, func: &dyn Fn(&Box<dyn std::error::Error + Send + Sync>)) {
        if let Self::Unprocessed(dynamic) = &self {
            func(dynamic)
        }
    }
}
