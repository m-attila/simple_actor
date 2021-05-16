use std::sync::Arc;

use tokio::sync::{Notify, Semaphore};

use crate::{ActorMessageClient, Res};
use crate::common::SimpleActorError;

/// Available synchronization types
enum Synchronization {
    Notify(Arc<Notify>),
    Semaphore(Arc<Semaphore>),
}

impl Synchronization {
    /// Waiting for synchronization objects
    async fn wait_for(&self) -> Res<()> {
        match self {
            Synchronization::Notify(s) => {
                s.notified().await;
                Ok(())
            }
            Synchronization::Semaphore(s) => {
                s.acquire().await.map_or_else(|_| Err(SimpleActorError::Receive.into()),
                                              |f| {
                                                  f.forget();
                                                  Ok(())
                                              })
            }
        }
    }

    /// Stop synchronization
    fn stop(self) {
        match self {
            Synchronization::Notify(_) => (),
            Synchronization::Semaphore(s) => s.close()
        }
    }
}

impl Clone for Synchronization {
    fn clone(&self) -> Self {
        match self {
            Synchronization::Notify(n) => Synchronization::Notify(Arc::clone(&n)),
            Synchronization::Semaphore(n) => Synchronization::Semaphore(Arc::clone(&n)),
        }
    }
}

/// Integrate `tokio::sync::Notify with message actors.
pub struct MessageActorNotify<ME> {
    /// Client of an message actor
    client: Option<Box<dyn ActorMessageClient<Message=ME> + Send + Sync>>,
    /// Stop notification
    stop: Arc<Notify>,
    /// Listened notification
    synchron: Synchronization,
    /// Message which will be sent to the actor when `notify` is notified
    message: ME,
}

impl<ME: Send + Sync + Clone + 'static> MessageActorNotify<ME> {
    /// Integration between actor's client and notification
    pub fn notify(client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>,
                  notify: Arc<Notify>,
                  message: ME) -> Self {
        MessageActorNotify::new(client, message, Synchronization::Notify(notify))
    }


    /// Integration between actor's client and semaphore
    pub fn semaphore(client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>,
                     semaphore: Arc<Semaphore>,
                     message: ME) -> Self {
        MessageActorNotify::new(client, message, Synchronization::Semaphore(semaphore))
    }

    /// Create new instance
    fn new(client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>, message: ME, sync: Synchronization) -> MessageActorNotify<ME> {
        MessageActorNotify {
            client: Some(client),
            stop: Arc::new(Notify::new()),
            synchron: sync,
            message,
        }
    }

    /// Start interface which will listen for notification and fire message when it was notified
    #[allow(unused_must_use)]
    pub fn start(&mut self) {
        let synchron = self.synchron.clone();
        let stop = Arc::clone(&self.stop);
        let client = self.client.take().unwrap();
        let message = self.message.clone();

        tokio::spawn(async move {
            loop {
                let new_message = message.clone();

                tokio::select! {
                    e = synchron.wait_for() => {
                        match e {
                            Ok(_) => {
                                match client.message(new_message).await {
                                    Ok(_) => (),
                                    Err(_) => break
                                }
                            },
                            Err(_) => break
                        }
                    }
                    _ = stop.notified() =>{
                        break
                    }
                }
            }
        });
    }

    /// Stop the interface
    pub fn stop(self) {
        self.synchron.stop();
        self.stop.notify_waiters();
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use tokio::sync::{Notify, Semaphore};

    use crate::{ActorBuilder, MessageHandler, RequestHandler, Res};
    use crate::actor::client::sync::message::MessageActorNotify;

    /// Available notifications
    #[derive(Debug, Clone)]
    enum Messages {
        /// Notify actor
        Notify
    }

    /// Available requests
    #[derive(Debug)]
    enum Requests {
        /// Get actor counter value
        Get
    }

    /// Available responses
    #[derive(Debug, PartialEq)]
    enum Responses {
        /// Return actor counter value
        Val(u32)
    }

    /// Test actor
    struct TestActor {
        counter: u32
    }

    #[async_trait]
    impl MessageHandler for TestActor {
        type Message = Messages;

        async fn process_message(&mut self, _message: Self::Message) -> Res<()> {
            // Only `Notify` message is available
            self.counter += 1;
            Ok(())
        }
    }

    #[async_trait]
    impl RequestHandler for TestActor {
        type Request = Requests;
        type Reply = Responses;

        async fn process_request(&mut self, _request: Self::Request) -> Res<Self::Reply> {
            // Only `Get` request is available
            Ok(Responses::Val(self.counter))
        }
    }

    #[test]
    fn notification_test() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(
            async {
                // Create hybrid actor
                let actor = ActorBuilder::new()
                    .one_shot()
                    .hybrid_actor(Box::new(TestActor { counter: 0 }))
                    .build();

                // Create notification
                let notify = Arc::new(Notify::new());

                // Bind notification to actor with given message
                let mut notify_interface = MessageActorNotify::notify(
                    actor.message_client(),
                    Arc::clone(&notify),
                    Messages::Notify);
                notify_interface.start();

                // Create semaphore
                let semaphore = Arc::new(Semaphore::new(0));

                // Bind semaphore to actor with same message
                let mut semaphore_interface = MessageActorNotify::semaphore(
                    actor.message_client(),
                    Arc::clone(&semaphore),
                    Messages::Notify);
                semaphore_interface.start();

                // Create client for this test
                let client = actor.client();

                // Initially, the counter is zero
                assert_eq!(Responses::Val(0), client.request(Requests::Get).await.unwrap());

                // Waiting a while, to ensure the notify_interface has started
                tokio::time::sleep(Duration::from_millis(100)).await;
                // Fire notification
                notify.notify_waiters();
                // Waiting a while to ensure notification and message has processed
                tokio::time::sleep(Duration::from_millis(100)).await;
                // The counter is incremented for notification's affect
                assert_eq!(Responses::Val(1), client.request(Requests::Get).await.unwrap());

                // Add 15 permits to semaphore
                semaphore.add_permits(10);
                semaphore.add_permits(5);

                // Waiting a while to ensure semaphore permits and messages has processed
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert_eq!(Responses::Val(1 + 15), client.request(Requests::Get).await.unwrap());

                // The semaphore closing does not cause error
                semaphore.close();
                // Fire notification agaion
                notify.notify_waiters();
                // Waiting a while...
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert_eq!(Responses::Val(1 + 15 + 1), client.request(Requests::Get).await.unwrap());

                // stop interfaces
                notify_interface.stop();
                semaphore_interface.stop();

                // stop actor
                actor.stop().await.unwrap();
            }
        )
    }
}