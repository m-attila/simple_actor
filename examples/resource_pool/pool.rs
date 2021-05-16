extern crate async_trait;
extern crate log;

use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use log::{info, warn};
use tokio::sync::Notify;

use simple_actor::{ActorBuilder, ActorHybridClient, MessageHandler, MessageScheduler, RequestHandler, Res, Scheduling};
use simple_actor::actor::server::actor::hybrid::HybridActor;
use simple_actor::common::{RequestExecution, SimpleActorError};

use crate::common::ResourceFactory;

/// Represents a pool resource
struct Resource<R> {
    /// Resource
    r: R,
    /// Timestamp of last using
    last_used: SystemTime,
}

impl<R> Resource<R> {
    /// Create new instance resource which will be stored in the poo
    fn new(r: R) -> Self {
        Self {
            r,
            last_used: SystemTime::now(),
        }
    }

    /// Returns the elapsed time from last using
    fn elapsed(&self) -> Duration {
        self.last_used.elapsed().unwrap_or_default()
    }
}

struct Pool<R> {
    /// How many persistent resources is stored in the pool
    persistent_cnt: usize,
    /// How many persistent + temporary resources is stored in the pool
    maximal_cnt: usize,
    /// How many resources is allocated currently
    allocated_cnt: usize,
    /// After idle time, the temporary resources will be dropped out from the pool
    idle_time: Duration,
    /// Resource factory
    factory: Box<dyn ResourceFactory<R>>,
    /// Store of the resources
    resources: VecDeque<Resource<R>>,
}

impl<R> Pool<R> {
    /// Create new pool
    pub fn new(persistent_cnt: usize, maximal_cnt: usize, idle_time: Duration, factory: Box<dyn ResourceFactory<R>>) -> Self {
        Self {
            persistent_cnt,
            maximal_cnt,
            allocated_cnt: 0,
            idle_time,
            factory,
            resources: VecDeque::with_capacity(256),
        }
    }

    /// Alloc new resources is there is free one in the pool
    pub async fn alloc(&mut self) -> Option<R> {
        loop {
            match self.resources.pop_front() {
                None => {
                    if self.allocated_cnt < self.maximal_cnt {
                        // No free resource, but it can be create new one
                        break self.factory.create().await
                            .ok()
                            .and_then(|r| {
                                self.allocated_cnt += 1;
                                Some(r)
                            });
                    } else {
                        // Too many resources exist
                        return None;
                    }
                }
                Some(r) => {
                    // There is free resource in the pool
                    if self.factory.check(&r.r).await {
                        // The resources is healthy
                        self.allocated_cnt += 1;
                        return Some(r.r);
                    } else {
                        warn!("Resource has dropped");
                    }
                }
            }
        }
    }

    /// Free previously allocated resource
    pub fn free(&mut self, r: R) {
        self.allocated_cnt -= 1;
        self.resources.push_back(Resource::new(r));
        self.cleanup()
    }

    /// Drops unnecessary temporary resources
    pub fn cleanup(&mut self) {
        let mut unnecessary_cnt =
            if self.resources.len() + self.allocated_cnt >= self.persistent_cnt {
                self.resources.len() + self.allocated_cnt - self.persistent_cnt
            } else { 0 };

        if unnecessary_cnt > 0 {
            // There are temporary resources in the pool
            while let Some(r) = self.resources.front() {
                if r.elapsed() >= self.idle_time {
                    // The resources has not used for a time...
                    info!("Temporary resource has dropped");
                    self.resources.remove(0);
                    unnecessary_cnt -= 1;
                    if unnecessary_cnt == 0 {
                        break;
                    }
                } else { break; }
            }
        }
    }
}

/// Available requests of the resource pool actor
enum Requests<R, T> {
    /// Allocate new resource and execute the given asynchronous function with it
    Execute(fn(R) -> Pin<Box<dyn Future<Output=(T, R)> + Send>>),
    /// The resource has allocated and assigned to function, call it
    ExecuteWithResource(Pin<Box<dyn Future<Output=(T, R)> + Send>>),
    /// Resource allocation was not succeeded, wait for an other one
    WaitForResource(Arc<Notify>, fn(R) -> Pin<Box<dyn Future<Output=(T, R)> + Send>>),
    /// The function has executed, reply its result to the client
    Executed(R, T),
}

impl<R, T: Debug> Debug for Requests<R, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Requests::Execute(_) => f.write_str("Operation has received"),
            Requests::ExecuteWithResource(_) => f.write_str("Operation is started with an allocated resource"),
            Requests::WaitForResource(_, _) => f.write_str("Operation is waiting for a free resource"),
            Requests::Executed(_, res) => f.write_fmt(format_args!("Operation has executed with result: {:?}", res))
        }
    }
}

#[derive(Debug)]
enum Responses<T> {
    Done(T),
}

/// The resource actor
struct ResourceActor<R, T> {
    /// The pool
    pool: Pool<R>,
    /// Notification for those task, which ones are waiting for new allocable resource
    notify_free: Arc<Notify>,
    result_type: PhantomData<T>,
}

impl<R, T> ResourceActor<R, T>
    where T: Send + Sync + 'static,
          R: Send + Sync + 'static {
    /// Create new actor instance
    fn new(persistent_cnt: usize, maximal_cnt: usize, idle_time: Duration, factory: Box<dyn ResourceFactory<R>>) -> Self {
        Self {
            pool: Pool::new(persistent_cnt, maximal_cnt, idle_time, factory),
            notify_free: Arc::new(Notify::new()),
            result_type: PhantomData,
        }
    }

    /// Execute the given operation on the allocated resource.
    /// This function is processed by separate task, asynchronously
    fn execution_with_resource(request: Requests<R, T>) -> Pin<Box<dyn Future<Output=Res<Requests<R, T>>> + Send>> {
        Box::pin(async {
            match request {
                Requests::ExecuteWithResource(operation) => {
                    // Resource has allocated, start the operation on it
                    let (result, resource) = operation.await;
                    Ok(Requests::Executed(resource, result))
                }
                Requests::WaitForResource(notify, operation) => {
                    // There are no free resources, wait for another one
                    notify.notified().await;
                    Ok(Requests::Execute(operation))
                }
                _ => Err(SimpleActorError::UnexpectedCommand.into())
            }
        })
    }
}

#[async_trait]
impl<R, T> RequestHandler for ResourceActor<R, T>
    where R: Send + Sync + 'static,
          T: Send + Sync + 'static {
    type Request = Requests<R, T>;
    type Reply = Responses<T>;

    async fn classify_request(&mut self, request: Self::Request) -> RequestExecution<Self::Request> {
        match request {
            // Execute request from the clients
            Requests::Execute(f) => {
                match self.pool.alloc().await {
                    // Resource has allocated, it can be call `f` function with `res` resource within an asynchronous task
                    Some(res) => RequestExecution::Async(Requests::ExecuteWithResource(f(res))),
                    // There are no allocable resources, it has to wait for a free resource within an asynchronous task
                    None => RequestExecution::Async(Requests::WaitForResource(Arc::clone(&self.notify_free), f))
                }
            }
            // s is a synchronous task
            s @ _ => RequestExecution::Sync(s)
        }
    }

    fn get_async_transformation(&self) -> Box<dyn Fn(Self::Request) -> Pin<Box<dyn Future<Output=Res<Self::Request>> + Send>> + Send + Sync> {
        // Return the function will executes asynchronous task and transforms its result to a new request to the actor
        Box::new(ResourceActor::execution_with_resource)
    }

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            // Synchronous request which is sent by the processing task of the `Execute_With_Resource` request
            Requests::Executed(resource, result) => {
                self.pool.free(resource);
                self.notify_free.notify_one();
                // Reply to client
                Ok(Responses::Done(result))
            }
            _ => Err(SimpleActorError::UnexpectedCommand.into())
        }
    }
}

#[derive(Debug, Clone)]
/// Available actor messages
enum Messages {
    /// Drop the unnecessary temporary resources from the pool
    Cleanup
}

#[async_trait]
impl<R, T> MessageHandler for ResourceActor<R, T>
    where R: Send + Sync + 'static,
          T: Send + Sync + 'static {
    type Message = Messages;

    async fn process_message(&mut self, message: Self::Message) -> Res<()> {
        match message {
            Messages::Cleanup => {
                self.pool.cleanup();
                Ok(())
            }
        }
    }
}

// impl<R, T> HybridHandler for ResourceActor<R, T>
//     where R: Send + Sync + 'static,
//           T: Send + Sync + 'static {
//     fn request_handler_ref(&self) -> &dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
//         self
//     }
//
//     fn request_handler_mut(&mut self) -> &mut dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
//         self
//     }
// }


/// Wraps client of the `ResourceActor` to hide inner processing requests which participate in resource allocation and asynchronous processing.
/// The clients can send `Execute` requests only through this implementation.
pub struct ResourcePoolClient<R, T>(Box<dyn ActorHybridClient<Messages, Requests<R, T>, Responses<T>>>);

impl<R, T> ResourcePoolClient<R, T>
    where R: Send,
          T: Send {
    /// Create new client for resource pool
    fn new(client: Box<dyn ActorHybridClient<Messages, Requests<R, T>, Responses<T>>>) -> Self {
        Self(client)
    }

    /// Execute operation on a resource which will be allocated by the pool
    pub async fn exec(&self, operation: fn(R) -> Pin<Box<dyn Future<Output=(T, R)> + Send>>) -> Res<T> {
        self.0.request(Requests::Execute(operation)).await.map(|r|
            match r {
                Responses::Done(t) => t
            })
    }
}

/// Wraps the resource pool actor
pub struct ResourcePool<R: Send + Sync, T: Send + Sync> {
    /// The actor
    actor: HybridActor<Messages, Requests<R, T>, Responses<T>>,
    /// The scheduler which triggers the cleanup of the temporary resources
    scheduler: MessageScheduler,
}

impl<R, T> ResourcePool<R, T>
    where
        R: Send + Sync + Debug + 'static,
        T: Send + Sync + Debug + 'static {
    /// Create new instance and start the actor
    pub fn new(persistent_cnt: usize,
               maximal_cnt: usize,
               idle_time: Duration,
               factory: Box<dyn ResourceFactory<R>>) -> Self {
        let actor = ActorBuilder::new()
            .name("resource_pool")
            .receive_buffer_size(2048)
            .one_shot()
            .hybrid_actor(Box::new(ResourceActor::<R, T>::new(persistent_cnt, maximal_cnt, idle_time, factory)))
            .build();
        let client = actor.message_client();
        let scheduler = MessageScheduler::new(Messages::Cleanup, Scheduling::Periodic(idle_time), client);
        Self {
            actor,
            scheduler,
        }
    }

    /// Create new client for the resource pool
    pub fn client(&self) -> ResourcePoolClient<R, T> {
        ResourcePoolClient::new(self.actor.client())
    }

    /// Stop the pool
    pub async fn stop(self) -> Res<()> {
        self.scheduler.abort();
        self.actor.stop().await
    }
}
