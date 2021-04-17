use async_trait::async_trait;

use crate::actor::server::common::ActorServerHandler;
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
impl<MR: Send, R: Send> ActorServerHandler for ActorRequestServerHandler<MR, R> {
    type Message = ();
    type Request = MR;
    type Reply = R;

    async fn process(&mut self, command: Command<(), Self::Request, Self::Reply>) -> Res<()> {
        if let Command::Request(message, reply_to) = command {
            // It can accept only requests
            let res = self.handler.process_request(message).await;
            if reply_to.send(res).is_err() {
                // Request processing was failed
                Err(SimpleActorError::Send.into())
            } else { Ok(()) }
        } else {
            Err(SimpleActorError::UnexpectedCommand.into())
        }
    }
}