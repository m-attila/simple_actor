extern crate async_trait;
extern crate simple_actor;
extern crate simple_logger;

use log::info;
use log::LevelFilter;
use simple_logger::SimpleLogger;

use simple_actor::ActorBuilder;

use crate::consumer::AvgCalculatorFunction;
use crate::consumer::Calculator;
use crate::consumer::SumCalculatorFunction;
use crate::producer::Producer;
use crate::producer::Replies;

mod common;

/// Producer module
mod producer {
    use async_trait::async_trait;

    use simple_actor::ActorHybridClient;
    use simple_actor::{RequestHandler, Res};

    use super::consumer;

    /// Producer requests
    #[derive(Debug)]
    pub enum Requests {
        /// Fill request
        Fill(u32, u32),
        /// Get summary request
        GetSum,
        /// Get average request
        GetAvg,
    }

    /// Producer replies
    #[derive(Debug)]
    pub enum Replies {
        /// Fill request was success
        Filled,
        /// Result of summary
        Sum(u128),
        /// Result of average request
        Avg(u128),
    }

    /// Producer
    pub struct Producer {
        /// Client of summary calculator consumer actor
        sum_consumer: Option<Box<dyn ActorHybridClient<consumer::Messages, consumer::Requests, consumer::Replies>>>,
        /// Client of average calculator consumer actor
        avg_consumer: Option<Box<dyn ActorHybridClient<consumer::Messages, consumer::Requests, consumer::Replies>>>,
    }

    impl Producer {
        /// Creates new producer actor
        pub fn new() -> Self {
            Producer { sum_consumer: None, avg_consumer: None }
        }

        /// Set summary calculator actor's client
        pub fn set_sum_client(&mut self, client: Box<dyn ActorHybridClient<consumer::Messages, consumer::Requests, consumer::Replies>>) {
            self.sum_consumer = Some(client);
        }

        /// Set average calculator actor's client
        pub fn set_avg_client(&mut self, client: Box<dyn ActorHybridClient<consumer::Messages, consumer::Requests, consumer::Replies>>) {
            self.avg_consumer = Some(client);
        }
    }

    /// Handle request of producer actor
    #[async_trait]
    impl RequestHandler for Producer {
        type Request = Requests;
        type Reply = Replies;

        async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
            match request {
                Requests::Fill(from, to) => {
                    let sc = &self.sum_consumer
                        .as_ref()
                        .map_or(Err("no consumer client"), |c| Ok(c))?;
                    let ac = &self.avg_consumer
                        .as_ref()
                        .map_or(Err("no average client"), |c| Ok(c))?;
                    sc.message(consumer::Messages::Clear).await?;
                    ac.message(consumer::Messages::Clear).await?;

                    for i in from..=to {
                        sc.message(consumer::Messages::Put(i)).await?;
                        ac.message(consumer::Messages::Put(i)).await?;
                    }
                    Ok(Replies::Filled)
                }
                Requests::GetSum => {
                    let cl = self.sum_consumer
                        .as_ref()
                        .map_or(Err("no consumer client"), |c| Ok(c))?;
                    cl.request(consumer::Requests::Calculate)
                        .await
                        .map(|consumer::Replies::Result(n)| Replies::Sum(n))
                }
                Requests::GetAvg => {
                    let cl = self.avg_consumer
                        .as_ref()
                        .map_or(Err("no average client"), |c| Ok(c))?;
                    cl.request(consumer::Requests::Calculate)
                        .await
                        .map(|consumer::Replies::Result(n)| Replies::Avg(n))
                }
            }
        }
    }
}

/// consumer module
pub mod consumer {
    use std::slice::Iter;

    use async_trait::async_trait;

    use simple_actor::{HybridHandler, MessageHandler, RequestHandler, Res};

    /// Consumer messages
    #[derive(Debug)]
    pub enum Messages {
        /// Clear numbers message
        Clear,
        /// Put number into storage message
        Put(u32),
    }

    #[derive(Debug)]
    pub enum Requests {
        /// Calculate request for stored numbers
        Calculate,
    }

    /// Consumer response
    #[derive(Debug)]
    pub enum Replies {
        /// Reply for Calculate request
        Result(u128),
    }

    /// Calculator base logic
    struct CalculatorBase(Vec<u32>);

    impl CalculatorBase {
        /// Create new calculator
        fn new() -> Self {
            Self(Vec::with_capacity(256))
        }

        /// Store number
        fn store(&mut self, n: u32) {
            self.0.push(n)
        }

        /// Clear stored numbers
        fn clear(&mut self) {
            self.0.clear();
        }

        /// Gets iterator
        fn iter(&self) -> std::slice::Iter<u32> {
            self.0.iter()
        }
    }

    /// This trait specifies the calculate function for calculator
    pub trait CalculatorFunction: Send + Sync + 'static {
        fn calculate(&self, iter: std::slice::Iter<u32>) -> u128;
    }

    /// Base implementation for calculator actors
    pub struct Calculator {
        calculator: CalculatorBase,
        calc_fun: Box<dyn CalculatorFunction>,
    }

    impl Calculator {
        /// Creates new instance
        pub fn new(func: Box<dyn CalculatorFunction>) -> Self {
            Calculator {
                calculator: CalculatorBase::new(),
                calc_fun: func,
            }
        }
    }

    #[async_trait]
    impl MessageHandler for Calculator {
        type Message = Messages;

        async fn process_message(&mut self, message: Self::Message) -> Res<()> {
            match message {
                Messages::Clear => self.calculator.clear(),
                Messages::Put(n) => self.calculator.store(n),
            }
            Ok(())
        }
    }

    #[async_trait]
    impl RequestHandler for Calculator {
        type Request = Requests;
        type Reply = Replies;

        async fn process_request(&mut self, request: Self::Request) -> Res<Self::Reply> {
            match request {
                Requests::Calculate => {
                    Ok(Replies::Result(self.calc_fun.calculate(self.calculator.iter())))
                }
            }
        }
    }

    #[async_trait]
    impl HybridHandler for Calculator {
        fn request_handler_ref(&self) -> &dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
            self
        }

        fn request_handler_mut(&mut self) -> &mut dyn RequestHandler<Request=Self::Request, Reply=Self::Reply> {
            self
        }
    }

    /// Sum calculator function
    pub struct SumCalculatorFunction;

    impl CalculatorFunction for SumCalculatorFunction {
        fn calculate(&self, iter: Iter<u32>) -> u128 {
            iter.map(|u| *u as u128).sum()
        }
    }

    /// Average calculator function
    pub struct AvgCalculatorFunction;

    impl CalculatorFunction for AvgCalculatorFunction {
        fn calculate(&self, iter: Iter<u32>) -> u128 {
            let mut cnt = 0u128;
            let mut sum: u128 = 0u128;

            for i in iter {
                cnt += 1;
                sum += *i as u128;
            }
            sum / cnt
        }
    }
}

#[test]
#[allow(unused_must_use)]
fn calc_test() {
    SimpleLogger::new().init();
    log::set_max_level(LevelFilter::Debug);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // Sum calculator logic
        let sum_calc = Box::new(Calculator::new(Box::new(SumCalculatorFunction {})));
        // Wraps into 'consumer' actor
        let sum_calc_actor = ActorBuilder::new()
            .name("SumCalculator")
            .one_shot()
            .hybrid_actor(sum_calc)
            .build();

        // Average calculator logic
        let avg_calc = Box::new(Calculator::new(Box::new(AvgCalculatorFunction {})));
        // Wraps into 'consumer' actor
        let avg_calc_actor = ActorBuilder::new()
            .name("AvgCalculator")
            .one_shot()
            .hybrid_actor(avg_calc)
            .build();

        // Producer logic
        let mut producer = Producer::new();
        // Sets consumers' clients
        producer.set_sum_client(sum_calc_actor.client());
        producer.set_avg_client(avg_calc_actor.client());

        // Producer actor
        let prod_actor = ActorBuilder::new()
            .name("Producer")
            .one_shot()
            .request_actor(Box::new(producer))
            .build();

        // Gets producer's client
        let prod_actor_cl = prod_actor.client();

        // Start producer process
        prod_actor_cl.request(producer::Requests::Fill(0, 100_000)).await.unwrap();

        // Request to get sums
        if let Replies::Sum(sums) = prod_actor_cl.request(producer::Requests::GetSum).await.unwrap() {
            info!("Sum is: {}", sums);
        }

        // Request to get averages
        if let Replies::Avg(avgs) = prod_actor_cl.request(producer::Requests::GetAvg).await.unwrap() {
            info!("Average is: {}", avgs);
        }

        // stop producer
        prod_actor.stop().await.unwrap();
        // stop consumers
        sum_calc_actor.stop().await.unwrap();
        avg_calc_actor.stop().await.unwrap();
    })
}
