//! Common types and structures for simple_actor.
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
    /// Unexpected command under the execution
    UnexpectedCommand,
}

impl std::fmt::Display for SimpleActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SimpleActorError::Receive => "Receive error",
            SimpleActorError::Send => "Send error",
            SimpleActorError::UnexpectedCommand => "Unexpected command",
        })
    }
}

impl std::error::Error for SimpleActorError {}

impl TryFrom<&ActorError> for SimpleActorError {
    type Error = &'static str;

    fn try_from(value: &ActorError) -> Result<Self, Self::Error> {
        match value.downcast_ref::<SimpleActorError>() {
            None => Err("Not a SimpleActorError type"),
            Some(res) => Ok(*res),
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

/// This trait should be implemented to process an asynchronous messages.
#[async_trait]
pub trait MessageHandler: Send {
    /// Type of the actor's message
    type Message: Send;

    /// Message processor.
    /// This method processes `message` argument and if the processing is succeeded, acknowledges it with [`enum@Result::Ok`].
    /// If method returns a wrapped [`type@ActorError`] value within [`enum@Result::Err`]  the actor will stop immediately.
    /// In this case, the actor's `stop` method will return with the error of the `process_message` function.
    async fn process_message(&mut self, _message: Self::Message) -> Res<()>;
}

/// Classified request. Indicates how can process a received request.
pub enum RequestExecution<R> {
    /// The request will be processed synchronously. Until the processing has finished,
    /// the actor does not step to the next message or request
    Sync(R),
    /// The request will be processed asynchronously. During its processing, the actor
    /// start to deal with a next message or request.
    Async(R),
    /// Works same as [`Async`](enum@RequestExecution::Async) execution, but it is a blocking processing, the actor start to deal
    /// with it within a new task. See [`spawn_blocking`](fn@tokio::task::spawn_blocking) method.
    Blocking(R),
}

/// This trait should be implemented to process a synchronous requests.
#[async_trait]
pub trait RequestHandler: Send {
    /// Type of request
    type Request: Send;
    /// Type of reply
    type Reply: Send;

    /// Classify the request by execution mode
    async fn classify_request(
        &mut self,
        request: Self::Request,
    ) -> RequestExecution<Self::Request> {
        RequestExecution::Sync(request)
    }

    /// Request processor.
    /// Process `request` argument and return with processing reply. The reply could be
    /// `Ok(:Reply)` or `Err(:ActorError)` which will be returned into the actor's client.
    async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply>;

    /// Returns a function which executes a blocking operation. This computation will not
    /// block any other receiving and processing mechanism. When the computation has finished successfully,
    /// this function returns a new request which is wrapped into the computation's result. The actor process this new
    /// request serially as any other requests. With this serialization method, the computation's result
    /// could change the actor's state.
    fn get_blocking_transformation(
        &self,
    ) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        error!("Please implement get_blocking_transformation(...) method in RequestHandler implementation");
        unimplemented!()
    }

    /// Works same as the [`get_blocking_transformation`](fn@self::RequestHandler::get_blocking_transformation) method, but the returned
    /// function will be executed by an asynchronous operation in a task which is started with
    /// [`spawn`](fn@tokio::spawn)
    #[allow(clippy::type_complexity)]
    fn get_async_transformation(
        &self,
    ) -> Box<
        dyn Fn(Self::Request) -> Pin<Box<dyn Future<Output = Res<Self::Request>> + Send>>
            + Send
            + Sync,
    > {
        error!("Please implement get_async_transformation(...) method in RequestHandler implementation");
        unimplemented!()
    }

    /// In this method can be handled those errors, which occurs when the sending of a reply was failed.
    fn reply_error(&self, _result: Res<Self::Reply>) {
        error!("Unable to send reply for the request. Please reimplement reply_error(...) method in RequestHandler implementation");
    }
}

/// This trait coalesces the message handling and the request processing capabilities.
#[async_trait]
pub trait HybridHandler: Send + MessageHandler + RequestHandler {
    /// Return a request handler
    fn request_handler_ref(
        &self,
    ) -> &dyn RequestHandler<Request = Self::Request, Reply = Self::Reply>;
    /// Return a mutable request handler
    fn request_handler_mut(
        &mut self,
    ) -> &mut dyn RequestHandler<Request = Self::Request, Reply = Self::Reply>;
    /// Return a message handler
    fn message_handler_ref(&self) -> &dyn MessageHandler<Message = Self::Message>;
    /// Return a mutable message handler
    fn message_handler_mut(&mut self) -> &mut dyn MessageHandler<Message = Self::Message>;
}

impl<T: 'static> HybridHandler for T
where
    T: MessageHandler + RequestHandler,
{
    fn request_handler_ref(
        &self,
    ) -> &dyn RequestHandler<Request = Self::Request, Reply = Self::Reply> {
        self
    }

    fn request_handler_mut(
        &mut self,
    ) -> &mut dyn RequestHandler<Request = Self::Request, Reply = Self::Reply> {
        self
    }

    fn message_handler_ref(&self) -> &dyn MessageHandler<Message = Self::Message> {
        self
    }

    fn message_handler_mut(&mut self) -> &mut dyn MessageHandler<Message = Self::Message> {
        self
    }
}

/// This trait should be implemented to handle the actor initialization and termination.
pub trait StateHandler: Send {
    /// Initialization.
    /// If this method returns with `Ok(())` the initialization was succeeded otherwise it was failed.
    /// If the initialization was failed, the actor stops immediately.
    fn init(&mut self, name: String) -> Res<()>;

    /// Actor terminate event.
    /// This method receives the reason of the termination in `reason` argument.
    fn terminate(&mut self, name: String, _reason: &Res<()>);
}

/// This enum describes that an error is already processed or isn't yet.
pub enum ActorErrorHandler<'a> {
    /// The boxed error isn't processed yet.
    #[allow(clippy::borrowed_box)]
    Unprocessed(&'a Box<dyn std::error::Error + Send + Sync>),
    /// The boxed error is already processed.
    Processed,
}

impl<'a> ActorErrorHandler<'a> {
    /// Create a new instance for dynamic boxed instance
    #[allow(clippy::borrowed_box)]
    pub fn new(error: &'a Box<dyn std::error::Error + Send + Sync>) -> Self {
        ActorErrorHandler::Unprocessed(error)
    }

    /// If the boxed error can be assigned to a `T` type, then the `func` function will process it.
    pub fn and_then<T: 'static + std::error::Error + Send + Sync>(
        &self,
        func: &dyn Fn(&T),
    ) -> &Self {
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

    /// Process the boxed error if it could not be assigned to `T` type.
    /// The `func` function could handle this case.
    pub fn or_else(&self, func: &dyn Fn(&Box<dyn std::error::Error + Send + Sync>)) {
        if let Self::Unprocessed(dynamic) = &self {
            func(dynamic)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate async_trait;

    use std::convert::TryFrom;

    use async_trait::async_trait;

    use crate::common::{ActorErrorHandler, RequestExecution, SimpleActorError};
    use crate::{ActorError, RequestHandler, Res};

    #[test]
    fn simple_actor_error() {
        assert_eq!("Receive error", format!("{}", SimpleActorError::Receive));
        assert_eq!("Send error", format!("{}", SimpleActorError::Send));
        assert_eq!(
            "Unexpected command",
            format!("{}", SimpleActorError::UnexpectedCommand)
        );
    }

    #[test]
    fn convert_from_actor_error() {
        let ae: ActorError = SimpleActorError::Receive.into();
        assert_eq!(
            SimpleActorError::Receive,
            SimpleActorError::try_from(&ae).unwrap()
        );
        let ae: ActorError = SimpleActorError::UnexpectedCommand.into();
        assert_eq!(
            SimpleActorError::UnexpectedCommand,
            SimpleActorError::try_from(&ae).unwrap()
        );
        let ae: ActorError = SimpleActorError::Send.into();
        assert_eq!(
            SimpleActorError::Send,
            SimpleActorError::try_from(&ae).unwrap()
        );
        let ae: ActorError = std::io::Error::from(std::io::ErrorKind::AddrInUse).into();
        assert_eq!(
            "Not a SimpleActorError type",
            SimpleActorError::try_from(&ae).unwrap_err()
        );
    }

    #[test]
    fn actor_error_handler() {
        let ae: ActorError = SimpleActorError::Receive.into();
        ActorErrorHandler::new(&ae)
            .and_then::<std::io::Error>(&|_f| panic!())
            .and_then::<SimpleActorError>(&|_f| println!("ok"))
            .or_else(&|_f| panic!());

        let ae: ActorError = SimpleActorError::Receive.into();
        ActorErrorHandler::new(&ae)
            .and_then::<std::io::Error>(&|_f| panic!())
            .or_else(&|_f| println!("ok"));
    }

    struct Test();

    #[async_trait]
    impl RequestHandler for Test {
        type Request = ();
        type Reply = ();

        async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply> {
            todo!()
        }
    }

    #[test]
    #[should_panic]
    #[allow(unused_must_use)]
    fn request_handler_unimpl_gbt() {
        let handler = Test();
        handler.get_blocking_transformation();
    }

    #[test]
    #[should_panic]
    #[allow(unused_must_use)]
    fn request_handler_unimpl_gat() {
        let handler = Test();
        handler.get_async_transformation();
    }

    #[test]
    fn request_handler_defaults() {
        let mut handler = Test();
        handler.reply_error(Ok(()));
        let res = futures::executor::block_on(handler.classify_request(()));
        match res {
            RequestExecution::Sync(()) => (),
            RequestExecution::Async(_) => panic!(),
            RequestExecution::Blocking(_) => panic!(),
        };
    }
}
