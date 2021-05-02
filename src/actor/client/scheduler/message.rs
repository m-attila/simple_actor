use std::sync::Arc;

use async_trait::async_trait;
use log::{error, info, trace, warn};
use tokio::sync::Mutex;

use crate::actor::client::message::ActorMessageClient;
use crate::actor::client::scheduler::common::{Scheduler, SchedulerEventHandler, Scheduling};
use crate::common::Res;

struct MessageActorScheduler<ME: Send>(Box<dyn ActorMessageClient<Message=ME> + Send + Sync>);

impl<ME> MessageActorScheduler<ME> where ME: Send {
    pub fn new(client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>) -> Self {
        info!("Message scheduler was created for actor: {}", client.name());
        Self(client)
    }
}

#[async_trait]
impl<ME> SchedulerEventHandler<ME> for MessageActorScheduler<ME>
    where ME: Send + Sync + Clone {
    async fn handle_timer_event(&mut self, event: &ME) -> Res<bool> {
        let message = (*event).clone();
        match self.0.message(message).await {
            Ok(_) => {
                trace!("Scheduled message was sent to `{}` actor", self.0.name());
                Ok(true)
            }
            Err(e) => {
                error!("Scheduled message causes an error in `{}` actor: `{:?}`", self.0.name(), e);
                Ok(false)
            }
        }
    }
}

pub struct MessageScheduler(Scheduler, String);

impl MessageScheduler {
    /// Creates message scheduler
    pub fn new<ME: Send + Sync + Clone + 'static>(message: ME, scheduling: Scheduling, client: Box<dyn ActorMessageClient<Message=ME> + Send + Sync>) -> Self {
        let name = client.name();
        info!("Message scheduler was created for `{}` actor", name);
        let handler = Arc::new(Mutex::new(MessageActorScheduler::new(client)));
        let scheduler = Scheduler::new(scheduling, message, handler);
        MessageScheduler(scheduler, name)
    }

    /// Graceful stop the scheduler
    pub async fn stop(self) -> Res<()> {
        let name = self.1;
        match self.0.stop().await {
            Ok(r) => {
                info!("Message scheduler was stopped with result '{:?}' for actor `{}`", r, name);
                Ok(())
            }
            Err(e) => {
                error!("Message scheduler was stopped with error `{:?}` for actor `{}`", e, name);
                Err(e)
            }
        }
    }

    /// Abort the scheduler
    pub fn abort(self) {
        warn!("Message scheduler was aborted for `{}` actor", self.1);
        self.0.abort()
    }
}