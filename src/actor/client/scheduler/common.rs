use std::sync::Arc;

use async_trait::async_trait;
use log::{error, info, trace, warn};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::Duration;

use crate::common::Res;

/// Type of the scheduling
#[derive(Debug)]
pub enum Scheduling {
    /// Periodic with given interval
    Periodic(time::Duration),
    /// Periodic until a given point of time
    PeriodicAt(time::Instant, time::Duration),
    /// Only once at a given point of time
    OnceAt(time::Instant),
}

/// This trait specifies how to handle scheduled event
#[async_trait]
pub trait SchedulerEventHandler<E: Send>: Send + Sync {
    /// Process scheduler event. Returns `false` if scheduling process have to stop.
    async fn handle_timer_event(&mut self, _event: &E) -> Res<bool>;
}

/// Store scheduler data
struct SchedulerData<E: Send> {
    event: E,
    stype: Scheduling,
    handler: Arc<Mutex<dyn SchedulerEventHandler<E>>>,
    interval: Option<time::Interval>,
}

impl<E: Send> SchedulerData<E> {
    fn new(event: E, stype: Scheduling, handler: Arc<Mutex<dyn SchedulerEventHandler<E>>>) -> Self {
        Self {
            event,
            stype,
            handler,
            interval: None,
        }
    }

    fn set_interval(&mut self, interval: time::Interval) {
        self.interval = Some(interval);
    }

    async fn tick(&mut self) {
        if let Some(i) = self.interval.as_mut() {
            i.tick().await;
        }
    }

    async fn looping(&mut self) -> Res<()> {
        self.set_interval(match self.stype {
            Scheduling::Periodic(duration) => time::interval(duration),
            Scheduling::PeriodicAt(start, duration) => time::interval_at(start, duration),
            Scheduling::OnceAt(start) => time::interval_at(start, Duration::from_nanos(1)),
        });

        trace!(
            "Scheduler loop has started with `{:?}` scheduling",
            &self.stype
        );

        loop {
            self.tick().await;
            trace!("Event has been fired");
            let mut x = self.handler.lock().await;
            let result = x.handle_timer_event(&self.event).await;
            match result {
                Ok(again) => {
                    trace!("Scheduled event has been handled successfully");
                    if !again {
                        trace!("Event handler has stopped the scheduler");
                        break Ok(());
                    }
                }
                Err(e) => {
                    error!(
                        "An error has been occurred when handling a scheduler event: `{:?}`",
                        e
                    );
                    break Err(e);
                }
            }
            if let Scheduling::OnceAt(_) = self.stype {
                trace!("The event is scheduled only once, thus the scheduler has stopped");
                break Ok(());
            }
        }
    }
}

/// Scheduler instance
pub struct Scheduler {
    thread_handle: JoinHandle<Res<()>>,
}

impl Scheduler {
    /// Start new scheduler
    pub fn new<E: 'static + Send + Sync>(
        stype: Scheduling,
        event: E,
        handler: Arc<Mutex<dyn SchedulerEventHandler<E>>>,
    ) -> Self {
        info!("Scheduler has been created: {:?}", stype);
        let handle = tokio::spawn(async move {
            let mut data = SchedulerData::new(event, stype, handler);
            data.looping().await
        });
        Scheduler {
            thread_handle: handle,
        }
    }

    /// Stop the scheduler gracefully
    pub async fn stop(self) -> Res<()> {
        match self.thread_handle.await {
            Ok(r) => {
                info!("Scheduler has stopped with the following result: `{:?}`", r);
                r
            }
            Err(e) => {
                error!("Scheduler has stopped with the following error: `{:?}`", e);
                Err(e.into())
            }
        }
    }

    /// Abort the scheduler
    pub fn abort(self) {
        self.thread_handle.abort();
        warn!("Scheduler has been aborted");
    }
}

#[cfg(test)]
mod tests {
    mod exclusive {
        use std::ops::Add;

        use async_trait::async_trait;
        use tokio::time::Instant;

        use super::super::*;

        enum Event {
            Tick,
            Tock,
        }

        struct TestCounter {
            counter: u32,
            max: u32,
        }

        #[async_trait]
        impl SchedulerEventHandler<Event> for TestCounter {
            async fn handle_timer_event(&mut self, event: &Event) -> Res<bool> {
                match event {
                    Event::Tick => {
                        if self.counter < self.max {
                            self.counter += 1
                        }
                    }
                    Event::Tock => println!("Counter : {}", self.counter),
                };
                Ok(self.counter < self.max)
            }
        }

        #[test]
        fn periodic_test() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tc1 = Arc::new(Mutex::new(TestCounter {
                counter: 0,
                max: 10,
            }));
            let tc2 = Arc::clone(&tc1);

            rt.block_on(async {
                let sc1 = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(100)),
                    Event::Tick,
                    tc1,
                );
                let sc2 = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(50)),
                    Event::Tock,
                    tc2,
                );

                sc1.stop().await.unwrap();
                sc2.abort();
            })
        }

        #[test]
        fn periodic_at() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tc1 = Arc::new(Mutex::new(TestCounter {
                counter: 0,
                max: 10,
            }));
            let tc2 = Arc::clone(&tc1);

            rt.block_on(async {
                let at = Instant::now().add(Duration::from_millis(500));
                let sc1 = Scheduler::new(
                    Scheduling::PeriodicAt(at, Duration::from_millis(100)),
                    Event::Tick,
                    tc1,
                );
                let sc2 = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(50)),
                    Event::Tock,
                    tc2,
                );

                sc1.stop().await.unwrap();
                sc2.abort();
            })
        }

        #[test]
        fn once() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tc1 = Arc::new(Mutex::new(TestCounter {
                counter: 0,
                max: 10,
            }));
            let tc2 = Arc::clone(&tc1);

            rt.block_on(async {
                let sc1 = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(100)),
                    Event::Tick,
                    tc1,
                );
                let at = Instant::now().add(Duration::from_millis(500));
                let sc2 = Scheduler::new(Scheduling::OnceAt(at), Event::Tock, tc2);

                sc1.stop().await.unwrap();
                sc2.abort();
            })
        }

        #[test]
        fn handler_has_failed() {
            struct FailedCounter {
                counter: u32,
            }

            #[async_trait]
            impl SchedulerEventHandler<Event> for FailedCounter {
                async fn handle_timer_event(&mut self, event: &Event) -> Res<bool> {
                    match event {
                        Event::Tick => {
                            self.counter += 1;
                            Ok(true)
                        }
                        Event::Tock => Err("Tock is invalid".into()),
                    }
                }
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            let tc1 = Arc::new(Mutex::new(FailedCounter { counter: 0 }));
            let tc2 = Arc::clone(&tc1);

            rt.block_on(async {
                let sc1 = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(100)),
                    Event::Tick,
                    tc1,
                );
                let at = Instant::now().add(Duration::from_millis(100));
                let sc2 = Scheduler::new(Scheduling::OnceAt(at), Event::Tock, tc2);

                sc2.stop().await.unwrap_err();
                sc1.abort();
            })
        }

        #[test]
        fn handler_has_chrased() {
            struct FailedCounter();

            #[async_trait]
            impl SchedulerEventHandler<Event> for FailedCounter {
                async fn handle_timer_event(&mut self, _event: &Event) -> Res<bool> {
                    panic!()
                }
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            let tc = Arc::new(Mutex::new(FailedCounter()));

            rt.block_on(async {
                let sc = Scheduler::new(
                    Scheduling::Periodic(Duration::from_millis(10)),
                    Event::Tick,
                    tc,
                );

                tokio::time::sleep(Duration::from_millis(50)).await;

                sc.stop().await.unwrap_err();
            })
        }
    }
}
