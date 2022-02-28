#![no_std]
extern crate alloc;
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
        // we check if there's a lock because some immediate executing futures screw this up
        if let Some(mut e) = DEFAULT_EXECUTOR.try_lock() {
            // poll everything because future is done and may have created conditions for something to finish
            e.poll_tasks()
        }
    }
}

impl<T> Pendable for Arc<Task<T>> {
    fn is_pending(&self) -> bool {
        let mut future = self.future.lock();
        // make a waker for our task
        let waker = waker_ref(&self);
        // poll our future and give it a waker
        let context = &mut Context::from_waker(&*waker);
        matches!(future.as_mut().poll(context), Poll::Pending)
    }
}

impl Executor {
    // Run async task
    pub fn run<T>(&mut self, future: Pin<Box<dyn Future<Output = T> + 'static + Send>>)
    where
        T: Send + 'static,
    {
        self.add_task(future);
        self.poll_tasks();
    }

    /// Add task for a future to the list of tasks
    fn add_task<T>(
        &mut self,
        future: Pin<Box<dyn Future<Output = T> + 'static + Send>>,
    )
    where
        T: Send + 'static,
    {
        // store our task
        let task = Arc::new(Task {
            future: Mutex::new(future),
        });
        self.tasks.push_back(Box::new(task));
    }

    // Poll all tasks on global executor
    fn poll_tasks(&mut self) {
        for _ in 0..self.tasks.len() {
            let task = self.tasks.pop_front().unwrap();
            if task.is_pending() {
                self.tasks.push_back(task);
            }
        }
    }
}

lazy_static! {
    static ref DEFAULT_EXECUTOR: Mutex<Executor> = Mutex::new(Executor::default());
}

pub fn run<T>(future: impl Future<Output = T> + 'static + Send)
where
    T: Send + 'static,
{
    DEFAULT_EXECUTOR.lock().run(Box::pin(future))
}
