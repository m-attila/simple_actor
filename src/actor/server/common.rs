use async_trait::async_trait;

use crate::common::{Command, Res};

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
                     _message: Command<Self::Message, Self::Request, Self::Reply>) -> Res<()>;
}
