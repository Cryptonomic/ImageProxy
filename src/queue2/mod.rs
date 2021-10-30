use std::collections::HashSet;

//use std::pin::Pin;
use std::sync::Arc;

//use std::thread;
//use std::time::{Duration, Instant};
use async_trait::async_trait;
use tokio::sync::{RwLock, Semaphore};

#[async_trait]
pub trait Task: Sync + Send {
    async fn complete(&mut self) -> ();
    fn get_id(&self) -> String;
}

type GenericError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait QueueState {
    async fn enqueue(&mut selfkey: &str) -> Result<(), GenericError>;
    async fn dequeu(key: &str) -> Result<(), GenericError>;
}
// ^^^^ nope
//

pub struct Queue {
    state: Arc<RwLock<HashSet<String>>>,
    permits: Arc<Semaphore>,
}

impl Queue {
    pub fn new(concurrency: usize) -> Self
where {
        {
            Self {
                permits: Arc::new(Semaphore::new(concurrency)),
                state: Default::default(),
            }
        }
    }

    async fn add_task(&self, task: &(impl Task + 'static)) -> bool {
        //let success = state.insert(task.get_id());
        self.state.write().await.insert(task.get_id())
    }

    async fn remove_task(task: &(impl Task + 'static), state: Arc<RwLock<HashSet<String>>>) {
        state.write().await.remove(&task.get_id());
    }

    pub async fn spawn(&self, mut task: impl Task + 'static) {
        let permit = self.permits.clone().acquire_owned().await;
        if let true = self.add_task(&task).await {
            // let permits = self.permits.clone();
            let state = self.state.clone();
            tokio::task::spawn(async move {
                task.complete().await;
                drop(permit);
                Queue::remove_task(&task, state).await;
            });
        }
    }

    pub async fn job_exists(&self, url: String) -> bool {
        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check for the
        // same failure twice.
        let state = self.state.read().await;
        state.contains(&url)
    }
}
/*
#[cfg(test)]
mod queuetest {

    use std::collections::VecDeque;
    use std::thread;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::Receiver;

    use super::*;

    #[derive(Clone, Debug)]
    struct TestFut {
        id: u32,
        sent_at: Instant,
        //completed_at: Instant,
        sender: Option<mpsc::Sender<TestFut>>,
        payload_ms: u64,
    }

    impl TestFut {
        pub fn new(
            id: u32,
            sent_at: Instant,
            sender: mpsc::Sender<TestFut>,
            payload_ms: u64,
        ) -> Self {
            /*
             * let payload_handle = thread::spawn(move || {
                  thread::sleep(Duration::from_millis(payload_ms));
              });
              payload_handle.join();

              let _ = sender
                  .send(Self {
                      id,
                      sent_at,
                      completed_at: Instant::now(),
                  })
                  .await;
            */
            let slf = Self {
                id,
                sent_at,
                sender: Some(sender),
                payload_ms,
            };

            println!("testfut created: {}", slf.get_id());
            slf
        }
    }

    #[async_trait]
    impl Task for TestFut {
        async fn complete(&mut self) -> () {
            println!("testfut starting completion: {}", self.get_id());
            let payload_ms = self.payload_ms.clone();
            let payload_handle = thread::spawn(move || {
                thread::sleep(Duration::from_millis(payload_ms));
            });
            payload_handle.join();

            if let Some(sender) = self.sender.take() {
                sender.send(self.clone()).await;
            } //
        }

        fn get_id(&self) -> String {
            self.id.to_string()
        }
    }
    async fn check(
        concurrency: usize,
        mut input: VecDeque<TestFut>,
        mut reciever: Receiver<TestFut>,
        mut check: &mut impl FnMut(VecDeque<TestFut>) -> (),
    ) {
        let mut result_queue: VecDeque<TestFut> = VecDeque::new();
        let queue = Queue::new(concurrency);

        while let Some(task) = input.pop_front() {
            println!("spawning task {}", task.get_id());
            queue.spawn(task).await;
        }

        while let Some(testfut) = reciever.recv().await {
            let this_id = testfut.id;

            println!("testfut completed: \n {:?}", testfut);
            result_queue.push_back(testfut);
        }

        tokio::task::block_in_place(move || {
            check(result_queue);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn nonblocking_property() {
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, mut receiver) = mpsc::channel(8);
        let n_tests: u32 = 5;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(
                id,
                Instant::now(),
                sender.clone(),
                if id == 1 { 1000 } else { 0 },
            );
            send_queue.push_back(task);
        });
        let mut blocking_check = |mut result: VecDeque<TestFut>| {
            if let Some(last) = result.pop_back() {
                println!("checking! \n testfut {:?}", last);
                assert!(
                    last.id == 1,
                    "last complete job id {} expected id 1",
                    last.id
                );
            }
        };

        drop(sender);
        check(2, send_queue, receiver, &mut blocking_check).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fifo_property() {
        println!("entering future test");
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, mut receiver) = mpsc::channel(8);
        let n_tests: u32 = 5;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(
                id,
                Instant::now(),
                sender.clone(),
                if id == 1 { 500 } else { 1 },
            );
            send_queue.push_back(task);
        });
        let mut fifo_check = |mut result: VecDeque<TestFut>| {
            let mut last_id = 0;
            while let Some(testfut) = result.pop_front() {
                println!("checking! \n testfut {:?}", testfut);
                assert!(
                    last_id < testfut.id,
                    "last_id {} is not < task id {}",
                    last_id,
                    testfut.id
                );
                last_id = testfut.id;
            }
        };

        drop(sender);
        check(1, send_queue, receiver, &mut fifo_check).await;
    }
}
*/

#[cfg(test)]
mod queuetest {

    use std::collections::VecDeque;
    use std::thread;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::Receiver;

    use super::*;

    #[derive(Clone, Debug)]
    struct TestFut {
        id: u32,
        pub created_at: Instant,
        pub completed_at: Option<Instant>,
        sender: Option<mpsc::Sender<TestFut>>,
        pub payload_ms: u64,
    }

    impl TestFut {
        pub fn new(
            id: u32,
            created_at: Instant,
            sender: mpsc::Sender<TestFut>,
            payload_ms: u64,
        ) -> Self {
            let slf = Self {
                id,
                created_at,
                completed_at: None,
                sender: Some(sender),
                payload_ms,
            };

            println!("testfut created: {}", slf.get_id());
            slf
        }
    }

    #[async_trait]
    impl Task for TestFut {
        async fn complete(&mut self) -> () {
            println!("testfut starting completion: {}", self.get_id());
            let payload_ms = self.payload_ms.clone();
            let payload_handle = thread::spawn(move || {
                thread::sleep(Duration::from_millis(payload_ms));
            });
            let _ = payload_handle.join();

            if let Some(sender) = self.sender.take() {
                let _ = sender
                    .send(Self {
                        completed_at: Some(Instant::now()),
                        ..self.clone()
                    })
                    .await;
            } //
        }

        fn get_id(&self) -> String {
            self.id.to_string()
        }
    }
    async fn check(
        concurrency: usize,
        mut input: VecDeque<TestFut>,
        mut reciever: Receiver<TestFut>,
        check: &mut impl FnMut(VecDeque<TestFut>) -> (),
    ) {
        let mut result_queue: VecDeque<TestFut> = VecDeque::new();
        let queue = Queue::new(concurrency);

        while let Some(task) = input.pop_front() {
            let id = task.get_id();

            println!("spawning task {}", id);
            queue.spawn(task).await;
            assert!(queue.job_exists(id).await);
        }

        while let Some(testfut) = reciever.recv().await {
            let id = testfut.get_id();
            assert!(!queue.job_exists(id).await);
            println!("testfut completed: \n {:?}", testfut);
            result_queue.push_back(testfut);
        }

        tokio::task::block_in_place(move || {
            check(result_queue);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn nonblocking_property() {
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, receiver) = mpsc::channel(1000);
        let n_tests: u32 = 500;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(
                id,
                Instant::now(),
                sender.clone(),
                if id == 1 { 500 } else { 0 },
            );
            send_queue.push_back(task);
        });
        let mut blocking_check = |mut result: VecDeque<TestFut>| {
            if let Some(last) = result.pop_back() {
                println!("checking! \n testfut {:?}", last);
                assert!(
                    last.id == 1,
                    "last complete job id {} expected id 1",
                    last.id
                );
            }
        };

        drop(sender);
        check(2, send_queue, receiver, &mut blocking_check).await;
    }

    fn rank(test: &TestFut, results: &VecDeque<TestFut>) -> (u32, u32) {
        //let  rank = 0;

        let created_rank = results.iter().fold(0, |rank, result| {
            if result.created_at < test.created_at {
                rank + 1
            } else {
                rank
            }
        });

        let completed_rank = results.iter().fold(0, |rank, result| {
            if result.completed_at.unwrap() < test.completed_at.unwrap() {
                rank + 1
            } else {
                rank
            }
        });
        (created_rank, completed_rank)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fifo_property() {
        println!("entering future test");
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, receiver) = mpsc::channel(1000);
        let n_tests: u32 = 1000;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(id, Instant::now(), sender.clone(), 1);
            send_queue.push_back(task);
        });
        let mut fifo_check = |result: VecDeque<TestFut>| {
            let ranks: VecDeque<(u32, u32)> = result.iter().map(|t| rank(t, &result)).collect();
            let mut r: VecDeque<(&TestFut, &(u32, u32))> =
                result.iter().zip(ranks.iter()).collect();

            // let mut last_id = 0;
            while let Some((testfut, (start_rank, complete_rank))) = r.pop_front() {
                println!(
                    "checking! \n testfut {}, start_rank: {} , complete_rank: {}",
                    testfut.id, start_rank, complete_rank
                );
                assert!(
                    start_rank == complete_rank,
                    "start_rank {} is not = complete_rank {} for test {:?}",
                    start_rank,
                    complete_rank,
                    testfut
                );
                //last_id = testfut.id;
            }
        };

        drop(sender);
        check(1, send_queue, receiver, &mut fifo_check).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_property() {
        println!("entering future test");
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, receiver) = mpsc::channel(1000);
        let n_tests: u32 = 50;
        let concurrency = 4usize;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(id, Instant::now(), sender.clone(), 1);
            send_queue.push_back(task);
        });
        let mut concurrent_check = |result: VecDeque<TestFut>| {
            let ranks: VecDeque<(u32, u32)> = result.iter().map(|t| rank(t, &result)).collect();
            let mut r: VecDeque<(&TestFut, &(u32, u32))> =
                result.iter().zip(ranks.iter()).collect();
            // let mut last_id = 0;

            while let Some((testfut, (start_rank, complete_rank))) = r.pop_front() {
                println!(
                    "checking! \n testfut {}, start_rank: {} , complete_rank: {}",
                    testfut.id, start_rank, complete_rank
                );
                use std::convert::TryInto;
                let cr: usize = (*complete_rank).try_into().unwrap();
                let sr: usize = (*start_rank).try_into().unwrap();

                assert!(
                    sr >= cr.checked_sub(concurrency - 1).unwrap_or(0) && sr < cr + concurrency,
                    "start_rank {} is not in range of  complete_rank({}) +/- concurreny({}) for test {:?}",
                    start_rank,
                    complete_rank,
                    concurrency,
                    testfut
                );
                //last_id = testfut.id;
            }
        };

        drop(sender);
        check(concurrency, send_queue, receiver, &mut concurrent_check).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic]
    async fn concurrent_property_fail() {
        println!("entering future test");
        let mut send_queue: VecDeque<TestFut> = VecDeque::new();

        let (sender, receiver) = mpsc::channel(1000);
        let n_tests: u32 = 500;
        let concurrency = 4usize;

        (1..n_tests).for_each(|id| {
            let task = TestFut::new(id, Instant::now(), sender.clone(), 1);
            send_queue.push_back(task);
        });
        let mut fifo_check = |result: VecDeque<TestFut>| {
            let ranks: VecDeque<(u32, u32)> = result.iter().map(|t| rank(t, &result)).collect();
            let mut r: VecDeque<(&TestFut, &(u32, u32))> =
                result.iter().zip(ranks.iter()).collect();

            // let mut last_id = 0;

            while let Some((testfut, (start_rank, complete_rank))) = r.pop_front() {
                println!(
                    "checking! \n testfut {}, start_rank: {} , complete_rank: {}",
                    testfut.id, start_rank, complete_rank
                );

                assert!(
                    start_rank == complete_rank,
                    "start_rank {} is not = complete_rank {} for test {:?}",
                    start_rank,
                    complete_rank,
                    testfut
                );
                //last_id = testfut.id;
            }
        };

        drop(sender);
        check(concurrency, send_queue, receiver, &mut fifo_check).await;
    }
}
