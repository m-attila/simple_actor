use std::fmt::Debug;

use log::{debug, error, trace};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::actor::server::common::ActorServerHandler;
use crate::common::{Command, Res, SimpleActorError, StateHandler};

/// Engine of actor server
pub(crate) struct ActorServer<ME, MR, R>
    where ME: Send,
          MR: Send,
          R: Send {
    name: String,
    sender: mpsc::Sender<Command<ME, MR, R>>,
    thread_handle: JoinHandle<Res<()>>,
}

impl<ME, MR, R> ActorServer<ME, MR, R>
    where ME: 'static + Send + Debug,
          MR: 'static + Send + Debug,
          R: 'static + Send + Debug {
    /// Create new actor server
    pub(crate) fn new(name: String,
                      svr_handler: impl ActorServerHandler<Message=ME, Request=MR, Reply=R> + 'static,
                      mut state_handler: Box<dyn StateHandler>, receive_buffer: usize) -> Self {
        let (sender,
            receiver) = mpsc::channel(receive_buffer);

        let internal_sender = sender.clone();
        let i_name = name.clone();

        let handle = tokio::spawn(async move {
            let exit_val = match state_handler.init(i_name.clone()) {
                Ok(_) => Self::looping(svr_handler, receiver, internal_sender, i_name.clone()).await,
                Err(e) => {
                    error!("`{}` actor initialization was failed: `{:?}`", i_name, e);
                    Err(e)
                }
            };
            state_handler.terminate(i_name.clone(), &exit_val);
            exit_val
        });
        ActorServer {
            name,
            sender,
            thread_handle: handle,
        }
    }

    /// Returns the name of the actor
    pub fn name(&self) -> String {
        String::from(&self.name)
    }

    /// Returns sender channel
    pub fn sender(&self) -> mpsc::Sender<Command<ME, MR, R>> {
        self.sender.clone()
    }

    /// Stop server
    pub async fn stop(self) -> Res<()> {
        let _ = self.sender.send(Command::Stop).await;
        match self.thread_handle.await {
            Ok(r) => r,
            Err(e) => Err(e.into())
        }
    }

    /// Message/request processing loop
    async fn looping(mut svr_handler: impl ActorServerHandler<Message=ME, Request=MR, Reply=R>,
                     mut receiver: mpsc::Receiver<Command<ME, MR, R>>,
                     sender: mpsc::Sender<Command<ME, MR, R>>,
                     name: String) -> Res<()> {
        debug!("`{}` actor's receiver loop was started", name);
        let res =
            loop {
                match receiver.recv().await {
                    None => break {
                        error!("`{}` actor's receiver channel was closed", name);
                        Err(SimpleActorError::Receive.into())
                    },
                    Some(Command::Stop) => break {
                        debug!("`{}` actor: stop command was received", name);
                        Ok(())
                    },
                    Some(cmd) => {
                        trace!("`{:?}` command was received in `{}` actor", cmd, name);
                        match svr_handler.process(cmd).await {
                            // Synchronous request was processed, reply was sent
                            Ok(None) => (),
                            // Asynchronous request with long heavy computation. Needs to wait for other thread result
                            Ok(Some(handle)) => {
                                // Own sender of the actor server
                                let sender_c = sender.clone();
                                let i_name = name.clone();

                                tokio::task::spawn(async move {
                                    trace!("Heavy computation was started in actor `{}`", i_name);
                                    match handle.await {
                                        // heavy task returns with transformed request, which contains the computing result.
                                        // It will be processed by synchronous request in the request handler.
                                        // With this method can be modify the state of the actor by result.
                                        Ok(cmd) => {
                                            trace!("Heavy computation was ready in actor `{}`", i_name);
                                            if let Some(command) = cmd {
                                                // Heavy computing was returned with a new request
                                                match command {
                                                    Command::Request(_, _) => {
                                                        // The transformed result will be send the actor server itself.
                                                        if sender_c.send(command).await.is_err() {
                                                            error!("`{}` actor unable to send transformed request to itself", i_name);
                                                            panic!("Actor server stopped")
                                                        }
                                                    }
                                                    Command::RequestReplyError(_, _) => {
                                                        // The transformed result will be send the actor server itself.
                                                        if sender_c.send(command).await.is_err() {
                                                            error!("`{}` actor unable to send transformed request to itself", i_name);
                                                            panic!("Actor server stopped")
                                                        }
                                                    }
                                                    _ => {
                                                        error!("Unexpected command was received in actor `{}`", i_name);
                                                        panic!("Unexpected command")
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            // TODO: how to handle heavy computation errors
                                            error!("Heavy computation processing was failed in actor `{}`: `{:?}`", i_name, e);
                                            panic!("heavy computation error")
                                        }
                                    }
                                });
                            }
                            Err(e) => break Err(e)
                        }
                    }
                }
            };
        debug!("`{}` actor's receiver loop was exited with result: `{:?}`", name, res);
        res
    }
}

/// Tests
#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;

    use crate::actor::server::common::{ActorServerHandler, ProcessResult};
    use crate::common::{ActorError, ActorErrorHandler, Command, Res, StateHandler};

    use super::*;

    /// Custom error type
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub enum CustomError {
        InitError,
        ProcessError,
    }

    impl std::fmt::Display for CustomError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CustomError::InitError => write!(f, "Init error"),
                CustomError::ProcessError => write!(f, "Process error")
            }
        }
    }

    impl std::error::Error for CustomError {}

    impl TryFrom<&ActorError> for CustomError {
        type Error = &'static str;

        fn try_from(value: &ActorError) -> Result<Self, Self::Error> {
            match value.downcast_ref::<CustomError>() {
                None => Err("Not CustomError type"),
                Some(res) => Ok(*res)
            }
        }
    }

    unsafe impl Send for CustomError {}

    unsafe impl Sync for CustomError {}

    /// State handler for test
    struct TestStateHandler {
        /// To check for init event
        init_called: Arc<Mutex<bool>>,
        /// Return value of initialization
        init_return: Res<()>,
        /// To check for terminate event
        terminate_result: Arc<Mutex<Option<Res<()>>>>,
    }

    impl TestStateHandler {
        fn new(init_called: Arc<Mutex<bool>>,
               exp_init_return: Res<()>,
               terminate_result: Arc<Mutex<Option<Res<()>>>>) -> Self {
            Self {
                init_called,
                init_return: exp_init_return,
                terminate_result,
            }
        }
    }

    impl StateHandler for TestStateHandler {
        fn init(&mut self, _name: String) -> Res<()> {
            let mut ptr = self.init_called.lock().unwrap();
            *ptr = true;
            std::mem::replace(&mut self.init_return, Ok(()))
        }

        fn terminate(&mut self, _name: String, reason: &Res<()>) {
            match reason {
                Ok(_) => *self.terminate_result.lock().unwrap() = Some(Ok(())),
                Err(e) => {
                    ActorErrorHandler::new(e)
                        .and_then::<CustomError>(&|e|
                            *self.terminate_result.lock().unwrap() = Some(Err(e.clone().into())))
                        .or_else(&|e|
                            *self.terminate_result.lock().unwrap() = Some(Err(e.to_string().into())))
                }
            };
        }
    }

    /// Server handler for test
    struct TestServerHandler {
        /// Expected return value of message processing
        process_error: Result<(), CustomError>
    }

    #[async_trait]
    impl ActorServerHandler for TestServerHandler {
        type Message = String;
        type Request = ();
        type Reply = ();

        async fn process(&mut self, _message: Command<Self::Message, Self::Request, Self::Reply>) -> ProcessResult<Self::Message, Self::Request, Self::Reply> {
            async {
                self.process_error.map_or_else(|e| Err(e.into()), |_| Ok(None))
            }.await
        }
    }


    #[test]
    fn message_actor_test() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Test: init succeeded, process success
        rt.block_on(
            async {
                // Settings
                let init_called = Arc::new(Mutex::new(false));
                let terminate_result = Arc::new(Mutex::new(None));
                let terminate_result_chk = terminate_result.clone();

                // Create custom state handler
                let test_state_handler = TestStateHandler::new(
                    init_called.clone(),
                    Ok(()),
                    terminate_result);

                // Create test actor server handler
                let test_svr_handler = TestServerHandler { process_error: Ok(()) };
                // Create actor server
                let actor_server = ActorServer::<String, (), ()>::new(
                    "noname".to_string(), test_svr_handler,
                    Box::new(test_state_handler), 32);

                // Get sender channel to the actor server
                let sender = actor_server.sender();

                // Send message
                sender.send(Command::Message("hello".to_string())).await.unwrap();

                // Stop actor server
                let exit_result = actor_server.stop().await;
                exit_result.unwrap();

                let ptr = terminate_result_chk.lock().unwrap();
                match &*ptr {
                    // if 'terminate' function was not called
                    None => panic!(),
                    // if 'terminate' function was called
                    Some(std) => {
                        match std {
                            // expect: terminate was succeeded
                            Ok(_) => {}
                            // if terminate was failed
                            Err(_) => { panic!() }
                        }
                    }
                }

                // expect: 'init' was called
                assert_eq!(true, *init_called.lock().unwrap());
            }
        );

        // Test: init succeeded, process failed
        rt.block_on(
            async {
                // Settings
                let init_called = Arc::new(Mutex::new(false));
                let terminate_result = Arc::new(Mutex::new(None));
                let terminate_result_chk = terminate_result.clone();

                // Create custom state handler
                let test_state_handler = TestStateHandler::new(
                    init_called.clone(),
                    Ok(()),
                    terminate_result);

                // Create test actor server handler
                let test_svr_handler = TestServerHandler { process_error: Err(CustomError::ProcessError) };

                // Create actor server
                let actor_server = ActorServer::<String, (), ()>::new(
                    "noname".to_string(),
                    test_svr_handler,
                    Box::new(test_state_handler), 32);

                // Get sender channel to the actor server
                let sender = actor_server.sender();

                // Send message
                sender.send(Command::Message("hello".to_string())).await.unwrap();

                // Unwrap custom error which was occurred in message processing
                let custom_error = actor_server.stop()
                    .await
                    .map_err(|e| CustomError::try_from(&e).unwrap());

                // Stop returns with custom error
                assert_eq!(CustomError::ProcessError, custom_error.unwrap_err());

                let ptr = terminate_result_chk.lock().unwrap().take();
                match ptr {
                    // if 'terminate' function was not called
                    None => panic!(),
                    // if 'terminate' function was called
                    Some(std) => {
                        // unwrap custom error
                        let custom_error = std.map_err(|e| CustomError::try_from(&e).unwrap());
                        // expect: stop returns with custom error
                        assert_eq!(CustomError::ProcessError, custom_error.unwrap_err());
                    }
                }

                // expect: 'init' was called
                assert_eq!(true, *init_called.lock().unwrap());
            }
        );

        // Test: init failed
        rt.block_on(
            async {
                // Settings
                let init_called = Arc::new(Mutex::new(false));
                let terminate_result = Arc::new(Mutex::new(None));
                let terminate_result_chk = terminate_result.clone();

                // Create custom state handler
                let test_state_handler = TestStateHandler::new(
                    init_called.clone(),
                    Err(CustomError::InitError.into()),
                    terminate_result);

                // Create test actor server handler
                let test_svr_handler = TestServerHandler { process_error: Ok(()) };

                // Create actor server
                let actor_server = ActorServer::<String, (), ()>::new(
                    "noname".to_string(),
                    test_svr_handler,
                    Box::new(test_state_handler), 32);

                // ensure initialization has finished in async thread
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Get sender channel to the actor server
                let sender = actor_server.sender();
                // expect: channel is already closed
                assert_eq!("channel closed", sender.send(Command::Message("hello".to_string())).await.unwrap_err().to_string());

                // Unwrap custom error which was occurred in initialization
                let custom_error = actor_server.stop()
                    .await
                    .map_err(|e| CustomError::try_from(&e).unwrap());
                // expect: stop returns with initialization error
                assert_eq!(CustomError::InitError, custom_error.unwrap_err());

                let ptr = terminate_result_chk.lock().unwrap().take();
                match ptr {
                    // if 'terminate' function was not called
                    None => panic!(),
                    // if 'terminate' function was called
                    Some(std) => {
                        // unwrap custom error
                        let custom_error = std.map_err(|e| CustomError::try_from(&e).unwrap());
                        // expect: stop returns with initialization error
                        assert_eq!(CustomError::InitError, custom_error.unwrap_err());
                    }
                }

                // expect: 'init' was called
                assert_eq!(true, *init_called.lock().unwrap());
            }
        );
    }
}