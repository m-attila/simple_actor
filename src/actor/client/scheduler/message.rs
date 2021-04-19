use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::actor::client::message::ActorMessageClient;
use crate::actor::client::scheduler::common::{Scheduler, SchedulerEventHandler, Scheduling};
use crate::common::Res;

struct MessageActorScheduler<ME: Send>(Box<dyn ActorMessageClient<Message=ME> + Send + Sync>);

impl<ME> MessageActorScheduler<ME> where ME: Send {
    pub fn new(client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>) -> Self {
        Self(client)
    }
}

#[async_trait]
impl<ME> SchedulerEventHandler<ME> for MessageActorScheduler<ME>
    where ME: Send + Sync + Clone {
    async fn handle_timer_event(&mut self, event: &ME) -> Res<bool> {
        let message = (*event).clone();
        match self.0.message(message).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false)
        }
    }
}

pub struct MessageScheduler(Scheduler);

impl MessageScheduler {
    /// Creates message scheduler
    pub fn new<ME: Send + Sync + Clone + 'static>(message: ME, scheduling: Scheduling, client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>) -> Self {
        let handler = Arc::new(Mutex::new(MessageActorScheduler::new(client)));
        let scheduler = Scheduler::new(scheduling, message, handler);
        MessageScheduler(scheduler)
    }

    /// Graceful stop the scheduler
    pub async fn stop(self) -> Res<()> {
        self.0.stop().await
    }

    /// Abort the scheduler
    pub fn abort(self) {
        self.0.abort()
    }
}