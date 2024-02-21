use std::thread::JoinHandle;

use log::info;
use tokio::{
    runtime::{self},
    sync::mpsc::{self, Sender},
};
use uuid::Uuid;

struct Executor {
    executor_thread: Option<JoinHandle<()>>,
    task_sender: Option<Sender<Task>>,
}

struct Task {
    url: String,
    id: Uuid,
}

impl Task {
    pub async fn run(&self) {
        println!("Hello");
    }
}

impl Executor {
    pub fn new(rt_name: &str, num_threads: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<Task>(64);

        let rt_name = String::from(rt_name);
        let thread = std::thread::spawn(move || {
            let runtime = runtime::Builder::new_multi_thread()
                .thread_name(rt_name.to_owned())
                .worker_threads(num_threads)
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                info!("Executor `{}` started, channel open for tasks", rt_name);
                while let Some(task) = rx.recv().await {
                    tokio::task::spawn(async move {
                        task.run().await;
                    });
                }
                info!(
                    "Executor `{}` shutting down, channel closed for tasks",
                    rt_name
                );
            });
        });

        Executor {
            executor_thread: Some(thread),
            task_sender: Some(tx),
        }
    }
}
