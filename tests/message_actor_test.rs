extern crate async_trait;
extern crate simple_actor;
extern crate simple_logger;

use async_trait::async_trait;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::time::Duration;

use simple_actor::{MessageHandler, Res};
use simple_actor::ActorBuilder;
use simple_actor::MessageScheduler;
use simple_actor::Scheduling;

use crate::common::{Number, NumError};

mod common;

/// test actor logic
struct TestActor {
    counter: Number
}

#[async_trait]
impl MessageHandler for TestActor {
    type Message = Number;

    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        if message == 0 {
            // Error will stop actor
            Err(Box::new(NumError { value: self.counter }))
        } else {
            self.counter += message;
            Ok(())
        }
    }
}

#[test]
#[allow(unused_must_use)]
fn message_actor_test() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // custom actor logic
        let instance = TestActor { counter: 0 };

        // wraps logic into message actor
        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .message_actor(Box::new(instance))
            .build();

        // gets client for actor
        let client = actor.client();

        // expected sum
        let mut sum: u128 = 0;

        // sends messages
        for _ in 1..=100_000u128 {
            sum += 1;
            client.message(1).await.unwrap();
        }

        // If message processing causes error, actor stops.
        // The error is available by return value of stop method.
        // Send invalid value: 0
        // The message sending was succeeded, so it returns with Ok()
        client.message(0).await.unwrap();

        // Gets exit value
        let exit = actor.stop().await;

        let err = exit.unwrap_err();

        // Unwrap custom error
        match err.downcast_ref::<NumError>() {
            Some(num) => assert_eq!(sum, num.value),
            None => panic!("isn't a NumError type!"),
        }
    })
}

#[test]
#[allow(unused_must_use)]
fn restartable_message_actor_test() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // wraps logic into message actor
        let builder = ActorBuilder::new()
            .name("TestActor")
            .reusable()
            .message_actor(|| Box::new(TestActor { counter: 0 }));
        let actor = builder.build();

        // gets client for actor
        let client = actor.client();

        // expected sum
        let mut sum: u128 = 0;

        // sends messages
        for _ in 1..=100_000u128 {
            sum += 1;
            client.message(1).await.unwrap();
        }

        // If message processing causes error, actor stops.
        // The error is available by return value of stop method.
        // Send invalid value: 0
        // The message sending was succeeded, so it returns with Ok()
        client.message(0).await.unwrap();

        // Gets exit value
        let exit = actor.stop().await;

        let err = exit.unwrap_err();

        // Unwrap custom error
        match err.downcast_ref::<NumError>() {
            Some(num) => assert_eq!(sum, num.value),
            None => panic!("isn't a NumError type!"),
        }

        // Send message after actor was stopped
        client.message(1).await.unwrap_err();

        ////////////////////////////////////////////////////////////////////////
        // build new actor with the builder, and perform previous tests again

        // Create new actor
        let actor = builder.build();

        // gets client for actor
        let client = actor.client();

        // expected sum
        let mut sum: u128 = 0;

        // sends messages
        for _ in 1..=100_000u128 {
            sum += 1;
            client.message(1).await.unwrap();
        }

        // If message processing causes error, actor stops.
        // The error is available by return value of stop method.
        // Send invalid value: 0
        // The message sending was succeeded, so it returns with Ok()
        client.message(0).await.unwrap();

        // Gets exit value
        let exit = actor.stop().await;

        let err = exit.unwrap_err();

        // Unwrap custom error
        match err.downcast_ref::<NumError>() {
            Some(num) => assert_eq!(sum, num.value),
            None => panic!("isn't a NumError type!"),
        }

    })
}

#[test]
#[allow(unused_must_use)]
fn scheduled_message_actor_test() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // custom actor logic
        let instance = TestActor { counter: 0 };

        // wraps logic into message actor
        let actor = ActorBuilder::new()
            .name("TestActor")
            .one_shot()
            .message_actor(Box::new(instance))
            .build();

        // gets client for actor
        let client = actor.client();
        let client1 = actor.client();

        // start actor message scheduler
        let message_scheduler = MessageScheduler::new(1u128, Scheduling::Periodic(Duration::from_millis(30)), client);

        // waiting for 100ms
        tokio::time::sleep(Duration::from_millis(100)).await;
        // abort scheduler
        message_scheduler.abort();

        // If message processing causes error, actor stops.
        // The error is available by return value of stop method.
        // Send invalid value: 0
        // The message sending was succeeded, so it returns with Ok()
        client1.message(0).await.unwrap();

        // Gets exit value
        let exit = actor.stop().await;

        let err = exit.unwrap_err();

        // Unwrap custom error
        match err.downcast_ref::<NumError>() {
            Some(num) => assert!(num.value >= 3),
            None => panic!("isn't a NumError type!"),
        }
    })
}