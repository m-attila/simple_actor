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

/// Simulated query execution time range in milliseconds
const QUERY_EXECUTION_TIME_MSEC: Range<u64> = 5..25;
/// The query will report error in given ratio
const QUERY_FAILED_RATION: (u32, u32) = (1, 100);
/// Range of the number of query's result lines
const QUERY_RESULT_LINES: Range<usize> = 1..500;

/// The simulated connection construction will report error in given ratio
const CONNECT_FAILED_RATION: (u32, u32) = (1, 200);
/// The connections which already built up, will report error after its allocation before usage
const CONNECTION_CLOSED_RATIO: (u32, u32) = (1, 200);
/// Duration range to build up new connection
const CONNECT_EXECUTION_TIME_MSEC: Range<u64> = 1..10;
/// The temporary connections will be closed if it was not used since the given time
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
            DbError::QueryError => write!(f, "Query error")
        }
    }
}

impl std::error::Error for DbError {}

unsafe impl Send for DbError {}

unsafe impl Sync for DbError {}

/// Implements a simulated database resource which can execute queries
#[derive(Debug)]
struct DatabaseResource {}

impl Drop for DatabaseResource {
    fn drop(&mut self) {
        info!("Connection was closed");
    }
}

/// Used for error simulation
fn random_failed(ratio: (u32, u32), error: DbError) -> Result<(), DbError> {
    let (numerator, denominator) = ratio;
    if rand::thread_rng().gen_ratio(numerator, denominator) {
        Err(error)
    } else { Ok(()) }
}

/// Used to simulate different execution times
async fn random_wait(msec_range: Range<u64>) {
    let msec = rand::thread_rng().gen_range(msec_range);
    tokio::time::sleep(Duration::from_millis(msec)).await;
}

impl DatabaseResource {
    /// Execute a query operation.
    /// The operation could be failed or succeeded. The execution time is simulated with tokio's sleep
    /// method with random waiting times.
    async fn query(&self) -> Result<Vec<String>, DbError> {
        if let Err(e) = random_failed(QUERY_FAILED_RATION, DbError::QueryError) {
            error!("Query has failed");
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

/// Implements a [`ResourceFactory`](struct@crate::pool::ResourceFactory) implementation to generate
/// [`DatabaseResource`](struct@DatabaseResource) instances
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

/// This introduces how can be use the resources pool from a client. It get an allocated database resource
/// and return with an asynchronous operation which works with it. This function will be send by request to
/// the resource pool, which will allocate a new resource and by execute this function generates the asynchronous operation.
/// After this, the operation will be executed by task. If the task will be finished, the pool get back from it the resource
/// and put back into the pool.
fn query(connecion: DatabaseResource) -> Pin<Box<dyn Future<Output=(Result<Vec<String>, DbError>, DatabaseResource)> + Send>> {
    Box::pin(async move {
        // Execute the query
        let ret = connecion.query().await;
        match ret {
            Ok(lines) => {
                // Return with the result and with the allocated resource to put back into the pool
                (Ok(lines), connecion)
            }
            Err(e) => {
                error!("Query was failed: {:?}", e);
                // Return with the error and with the allocated resource to put back into the pool
                (Err(e), connecion)
            }
        }
    })
}

#[allow(unused_must_use)]
#[tokio::main]
pub async fn main() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Info);

    // Create the pool
    let pool =
        ResourcePool::<DatabaseResource, Result<Vec<String>, DbError>>::new(10,
                                                                            50,
                                                                            CONNECTION_IDLE_TIME,
                                                                            Box::new(DatabaseResourceFactory {}));
    // The request will be placed by futures into this vector, so all request runs concurrently
    let mut requests = Vec::new();
    // Start concurrent requests
    for _ in 0..5000 {
        let client = pool.client();
        let b = async move {
            client.exec(query).await
        };
        requests.push(b);
    };

    // wait for all requests has finished
    join_all(requests).await;

    // wait a time until the pool drops all temporary resources
    tokio::time::sleep(CONNECTION_IDLE_TIME.add(Duration::from_millis(50))).await;

    // start the request again
    let mut requests = Vec::new();
    for _ in 0..5000 {
        let client = pool.client();
        let b = async move {
            client.exec(query).await
        };
        requests.push(b);
    };
    join_all(requests).await;

    // stop the pool
    pool.stop().await;
}