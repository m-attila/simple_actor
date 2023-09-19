extern crate async_trait;
extern crate simple_actor;
extern crate tokio;

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use simple_actor::common::RequestExecution;
use simple_actor::{ActorBuilder, RequestHandler, Res};

#[derive(Debug)]
enum ActorRequests {
    Add(u32, u32),
    BlockingAdd(u32, u32),
    BlockingAddResult(u32),
    AsyncAdd(u32, u32),
    AsyncAddResult(u32),
}

#[derive(Debug)]
enum ActorResponses {
    Result(u32),
}

struct TestActor();

impl TestActor {
    fn blocking_add(req: ActorRequests) -> Res<ActorRequests> {
        match req {
            ActorRequests::BlockingAdd(x, y) => Ok(ActorRequests::BlockingAddResult(x + y)),
            _ => unreachable!(),
        }
    }

    fn async_add(req: ActorRequests) -> Pin<Box<dyn Future<Output = Res<ActorRequests>> + Send>> {
        Box::pin(async move {
            match req {
                ActorRequests::AsyncAdd(x, y) => Ok(ActorRequests::AsyncAddResult(x + y)),
                _ => unreachable!(),
            }
        })
    }
}

#[async_trait]
impl RequestHandler for TestActor {
    type Request = ActorRequests;
    type Reply = ActorResponses;

    async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
        match request {
            ActorRequests::Add(x, y) => Ok(ActorResponses::Result(x + y)),
            ActorRequests::BlockingAddResult(res) => Ok(ActorResponses::Result(res)),
            ActorRequests::AsyncAddResult(res) => Ok(ActorResponses::Result(res)),
            _ => unreachable!(),
        }
    }

    async fn classify_request(
        &mut self,
        request: Self::Request,
    ) -> RequestExecution<Self::Request> {
        match request {
            s @ ActorRequests::Add(_, _) => RequestExecution::Sync(s),
            a @ ActorRequests::BlockingAdd(_, _) => RequestExecution::Blocking(a),
            s @ ActorRequests::BlockingAddResult(_) => RequestExecution::Sync(s),
            a @ ActorRequests::AsyncAdd(_, _) => RequestExecution::Async(a),
            s @ ActorRequests::AsyncAddResult(_) => RequestExecution::Sync(s),
        }
    }

    fn get_blocking_transformation(
        &self,
    ) -> Box<dyn Fn(Self::Request) -> Res<Self::Request> + Send> {
        Box::new(TestActor::blocking_add)
    }

    fn get_async_transformation(
        &self,
    ) -> Box<
        dyn Fn(Self::Request) -> Pin<Box<dyn Future<Output = Res<Self::Request>> + Send>>
            + Send
            + Sync,
    > {
        Box::new(TestActor::async_add)
    }
}

pub fn request_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("requests");

    group.bench_function("req_actor_sync", |b| {
        b.to_async(&rt).iter_batched(
            || {
                let actor = ActorBuilder::new()
                    .receive_buffer_size(10240)
                    .one_shot()
                    .request_actor(Box::new(TestActor()))
                    .build();
                let client = actor.client();
                (actor, client)
            },
            |(_actor, client)| async move { client.request(ActorRequests::Add(10, 20)).await },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("req_actor_block", |b| {
        b.to_async(&rt)
            .iter_batched(|| {
                let actor = ActorBuilder::new()
                    .receive_buffer_size(10240)
                    .one_shot()
                    .request_actor(Box::new(TestActor()))
                    .build();
                let client = actor.client();
                (actor, client)
            },
                          |(_actor, client)|
                              async move {
                                  client.request(ActorRequests::BlockingAdd(10, 20)).await
                              }
                          , BatchSize::SmallInput,
            )
    });

    group.bench_function("req_actor_async", |b| {
        b.to_async(&rt).iter_batched(
            || {
                let actor = ActorBuilder::new()
                    .receive_buffer_size(10240)
                    .one_shot()
                    .request_actor(Box::new(TestActor()))
                    .build();
                let client = actor.client();
                (actor, client)
            },
            |(_actor, client)| async move { client.request(ActorRequests::AsyncAdd(10, 20)).await },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, request_benchmark);
criterion_main!(benches);
