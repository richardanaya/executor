#![no_std]
extern crate alloc;
use lazy_static::*;
use {
    alloc::{boxed::Box, sync::Arc, collections::vec_deque::VecDeque},
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
    spin::Mutex,
    woke::{waker_ref, Woke},
};

/// Executor holds a list of tasks to be processed
pub struct Executor {
    tasks: VecDeque<Arc<Task>>,
}

impl Default for Executor {
    fn default() -> Self {
        Executor {
            tasks: VecDeque::new(),
        }
    }
}

/// Task is our unit of execution and holds a future are waiting on
struct Task {
    pub future: Mutex<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
}

/// Implement what we would like to do when a task gets woken up
impl Woke for Task {
    fn wake_by_ref(_: &Arc<Self>) {
        // poll everything because future is done and may have created conditions for something to finish
        DEFAULT_EXECUTOR.lock().poll_tasks()
    }
}

impl GlobalExecutor for Executor {
    // Add a task on the global executor
    fn spawn(&mut self, future: Box<dyn Future<Output = ()> + 'static + Send + Unpin>) {
        self.add_task(future);
        self.poll_tasks();
    }
}

impl Executor {
    /// Add task for a future to the list of tasks
    fn add_task(&mut self, future: Box<dyn Future<Output = ()> + 'static + Send + Unpin>) {
        // store our task
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
        });
        self.tasks.push_back(task);
    }

    // Poll all tasks on global executor
    fn poll_tasks(&mut self) {
        let count = self.tasks.len();
        for _ in 0..count {
            let task = self.tasks.remove(0).unwrap();
            let mut is_pending = false;
            {
                let mut future = task.future.lock();
                // make a waker for our task
                let waker = waker_ref(&task);
                // poll our future and give it a waker
                let context = &mut Context::from_waker(&*waker);
                if let Poll::Pending = future.as_mut().poll(context) {
                    is_pending = true;
                }
            }
            if is_pending {
                self.tasks.push_back(task);
            }
        }
    }
}

struct DefaultExecutor;

impl GlobalExecutor for DefaultExecutor {
    // Add a task on the global executor
    fn spawn(&mut self, future: Box<dyn Future<Output = ()> + 'static + Send + Unpin>) {
        DEFAULT_EXECUTOR.lock().spawn(future)
    }
}


lazy_static! {
    static ref DEFAULT_EXECUTOR: Mutex<Box<Executor>> = {
        let m = Executor::default();
        Mutex::new(Box::new(m))
    };

    static ref GLOBAL_EXECUTOR: Mutex<Box<dyn GlobalExecutor+Send+Sync>> = {
        let m = DefaultExecutor;
        Mutex::new(Box::new(m))
    };
}

/// Give future to global executor to be polled and executed.
pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    GLOBAL_EXECUTOR.lock().spawn(Box::new(Box::pin(future)));
}

// Replace the default global executor with another
pub fn set_global_executor(executor:impl GlobalExecutor+Send+Sync+'static) {
    let mut global_executor = GLOBAL_EXECUTOR.lock();
    *global_executor = Box::new(executor);
}

pub trait GlobalExecutor {
    fn spawn(&mut self, future: Box<dyn Future<Output = ()> + 'static + Send + Unpin>);
}
