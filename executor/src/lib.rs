#![no_std]
extern crate alloc;
pub use executor_macros::*;
use lazy_static::*;
use {
    alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc},
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
    tasks: VecDeque<Box<dyn Pendable + core::marker::Send + core::marker::Sync>>,
}

trait Pendable {
    fn is_pending(&self) -> bool;
}

impl Default for Executor {
    fn default() -> Self {
        Executor {
            tasks: VecDeque::new(),
        }
    }
}

/// Task is our unit of execution and holds a future are waiting on
struct Task<T> {
    pub future: Mutex<Pin<Box<dyn Future<Output = T> + Send + 'static>>>,
}

/// Implement what we would like to do when a task gets woken up
impl<T> Woke for Task<T> {
    fn wake_by_ref(_: &Arc<Self>) {
        // poll everything because future is done and may have created conditions for something to finish
        DEFAULT_EXECUTOR.lock().poll_tasks()
    }
}

impl<T> Pendable for Arc<Task<T>> {
    fn is_pending(&self) -> bool {
        let mut future = self.future.lock();
        // make a waker for our task
        let waker = waker_ref(&self);
        // poll our future and give it a waker
        let context = &mut Context::from_waker(&*waker);
        let check_pending = matches!(future.as_mut().poll(context), Poll::Pending);
        check_pending
    }
}

impl Executor {
    // Add a task on the global executor
    fn block_on<T>(&mut self, future: Box<dyn Future<Output = T> + 'static + Send + Unpin>) -> T
    where
        T: Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
        });
        loop {
            let mut future = task.future.lock();
            // make a waker for our task
            let waker = waker_ref(&task);
            // poll our future and give it a waker
            let context = &mut Context::from_waker(&*waker);
            let result = future.as_mut().poll(context);
            if let Poll::Ready(val) = result {
                return val;
            }
        }
    }

    /// Add task for a future to the list of tasks
    /*fn add_task<T>(
        &mut self,
        future: Box<dyn Future<Output = T> + 'static + Send + Unpin>,
    ) -> Arc<Task<T>>
    where
        T: Send + 'static,
    {
        // store our task
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
        });
        self.tasks.push_back(Box::new(task.clone()));
        task
    }*/

    // Poll all tasks on global executor
    fn poll_tasks(&mut self) {
        let count = self.tasks.len();
        for _ in 0..count {
            let task = self.tasks.remove(0).unwrap();
            let mut is_pending = false;
            {
                if task.is_pending() {
                    is_pending = true;
                }
            }
            if is_pending {
                self.tasks.push_back(task);
            }
        }
    }
}

lazy_static! {
    static ref DEFAULT_EXECUTOR: Mutex<Box<Executor>> = {
        let m = Executor::default();
        Mutex::new(Box::new(m))
    };
}

/// Give future to global executor to be polled and executed.
pub fn block_on<T>(future: impl Future<Output = T> + 'static + Send) -> T
where
    T: Send + 'static,
{
    DEFAULT_EXECUTOR.lock().block_on(Box::new(Box::pin(future)))
}
