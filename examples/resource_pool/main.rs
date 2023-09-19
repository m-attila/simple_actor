//! This example introduces a resource pool handler, which accepts operations as functions, allocates a database connection resource
//! with which the operation can work.
//! The handler executes the operation asynchronously and after the execution puts the resource back to the pool.
extern crate async_trait;
extern crate futures;
extern crate log;
extern crate rand;
extern crate simple_logger;

use std::future::Future;
use std::ops::{Add, Range};
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::future::join_all;
use log::{error, info, LevelFilter};
use rand::Rng;
use simple_logger::SimpleLogger;

use crate::common::{Error, ResourceFactory};
use crate::pool::ResourcePool;

/// To generate a randomized query execution time.
const QUERY_EXECUTION_TIME_MSEC: Range<u64> = 5..25;
/// A likelihood that the query operation will be failed.
const QUERY_FAILED_RATION: (u32, u32) = (1, 100);
/// Number of rows in the query's result. (min, max)
const QUERY_RESULT_LINES: Range<usize> = 1..500;
/// A likelihood of a failed connection to database.
const CONNECT_FAILED_RATION: (u32, u32) = (1, 200);
/// A likelihood that a connection which is already built up, will be broken.
const CONNECTION_CLOSED_RATIO: (u32, u32) = (1, 200);
/// A range of time which necessary to build a new connection.
const CONNECT_EXECUTION_TIME_MSEC: Range<u64> = 1..10;
/// A duration which if is elapsed then the temporary connection will be disposed.
const CONNECTION_IDLE_TIME: Duration = Duration::from_millis(100);

mod common;
mod pool;

// Error response from actor
#[derive(Debug)]
pub enum DbError {
    UnableToConnect,
    ConnectionClosed,
    QueryError,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::UnableToConnect => write!(f, "Unable to connect to database"),
            DbError::ConnectionClosed => write!(f, "Connection closed"),
            DbError::QueryError => write!(f, "Query error"),
        }
    }
}

impl std::error::Error for DbError {}

unsafe impl Send for DbError {}

unsafe impl Sync for DbError {}

/// Implement a simulated database resource which can execute queries
#[derive(Debug)]
struct DatabaseResource {}

impl Drop for DatabaseResource {
    fn drop(&mut self) {
        info!("Connection has been closed");
    }
}

/// Random error simulation
fn random_failed(ratio: (u32, u32), error: DbError) -> Result<(), DbError> {
    let (numerator, denominator) = ratio;
    if rand::thread_rng().gen_ratio(numerator, denominator) {
        Err(error)
    } else {
        Ok(())
    }
}

/// To simulate different execution times
async fn random_wait(msec_range: Range<u64>) {
    let msec = rand::thread_rng().gen_range(msec_range);
    tokio::time::sleep(Duration::from_millis(msec)).await;
}

impl DatabaseResource {
    /// Execute a query operation.
    /// The operation could be failed or succeeded. The execution time is simulated with tokio's sleep.
    async fn query(&self) -> Result<Vec<String>, DbError> {
        if let Err(e) = random_failed(QUERY_FAILED_RATION, DbError::QueryError) {
            error!("Query has been failed");
            return Err(e);
        }

        random_wait(QUERY_EXECUTION_TIME_MSEC).await;

        let execution_time_msec: u64 = rand::thread_rng().gen_range(QUERY_EXECUTION_TIME_MSEC);
        tokio::time::sleep(Duration::from_millis(execution_time_msec)).await;

        let query_length = rand::thread_rng().gen_range(QUERY_RESULT_LINES);
        let mut result = Vec::with_capacity(query_length);
        for i in 0..query_length {
            result.push(format!("{}. result line", i))
        }
        Ok(result)
    }
}

/// Implement a [`ResourceFactory`](struct@crate::pool::ResourceFactory) to generate
/// [`DatabaseResource`](struct@DatabaseResource) instances.
struct DatabaseResourceFactory {}

#[async_trait]
impl ResourceFactory<DatabaseResource> for DatabaseResourceFactory {
    async fn create(&self) -> Result<DatabaseResource, Error> {
        random_failed(CONNECT_FAILED_RATION, DbError::UnableToConnect)?;
        random_wait(CONNECT_EXECUTION_TIME_MSEC).await;
        Ok(DatabaseResource {})
    }

    async fn check(&self, _resource: &DatabaseResource) -> bool {
        random_failed(CONNECTION_CLOSED_RATIO, DbError::ConnectionClosed).is_ok()
    }
}

/// This function introduces how a client can use the resources pool.
///
/// The function receives an allocated database connection and produces an asynchronous task which can execute a
/// query operation on this connection. This function will be sent as a request to the resource pool.
/// When the request has been delivered to the pool, it tries to allocate a free connection then calls this function to produce a query task.
/// This task will be executed asynchronously. After the task has finished, the pool get back the connection.
fn query(
    connecion: DatabaseResource,
) -> Pin<Box<dyn Future<Output = (Result<Vec<String>, DbError>, DatabaseResource)> + Send>> {
    Box::pin(async move {
        // Execute the query
        let ret = connecion.query().await;
        match ret {
            Ok(lines) => {
                // Return the result and the allocated resource to put back into the pool
                (Ok(lines), connecion)
            }
            Err(e) => {
                error!("Query was failed: {:?}", e);
                // Return the error and the allocated resource to put back into the pool
                (Err(e), connecion)
            }
        }
    })
}

#[allow(unused_must_use)]
#[tokio::main]
pub async fn main() {
    main_internal().await
}

async fn main_internal() {
    SimpleLogger::new().init().unwrap();
    log::set_max_level(LevelFilter::Info);

    // Create the pool
    let pool = ResourcePool::<DatabaseResource, Result<Vec<String>, DbError>>::new(
        10,
        50,
        CONNECTION_IDLE_TIME,
        Box::new(DatabaseResourceFactory {}),
    );
    // All requests will be placed as a task into this vector, thus all requests run concurrently
    let mut requests = Vec::new();
    // Start concurrent requests
    for _ in 0..5000 {
        let client = pool.client();
        let b = async move { client.exec(query).await };
        requests.push(b);
    }

    // wait for all requests are finished
    join_all(requests).await;

    // wait a time until the pool drops all temporary resources
    tokio::time::sleep(CONNECTION_IDLE_TIME.add(Duration::from_millis(50))).await;

    // start the request generation again
    let mut requests = Vec::new();
    for _ in 0..5000 {
        let client = pool.client();
        let b = async move { client.exec(query).await };
        requests.push(b);
    }
    join_all(requests).await;

    // stop the pool
    let _ = pool.stop().await;
}

#[tokio::test]
async fn my_test() {
    main_internal().await
}
