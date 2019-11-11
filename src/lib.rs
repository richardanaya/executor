#![no_std]
extern crate alloc;
use {
    alloc::{boxed::Box, sync::Arc},
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
    spin::Mutex,
    woke::{waker_ref, Woke},
};

use smallvec::*;

/// Executor holds a list of tasks to be processed
pub struct Executor {
    /// Tasks as a smallvec to try to keep it in the stack for as long as possible
    tasks: SmallVec<[Arc<Task>; 64]>,
}

impl Default for Executor {
    fn default() -> Self {
        Executor {
            tasks: SmallVec::new(),
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
        Executor::run()
    }
}

impl Executor {
    // Add a task on the global executor
    pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
        let mut e = globals::get::<Executor>().lock();
        e.add_task(future);
        e.poll_tasks();
    }

    // Poll all tasks on global executor
    fn run() {
        let mut e = globals::get::<Executor>().lock();
        Executor::poll_tasks(&mut e);
    }

    /// Add task for a future to the list of tasks
    fn add_task(&mut self, future: impl Future<Output = ()> + 'static + Send) {
        // store our task
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
        });
        self.tasks.push(task);
    }

    /// For every current task, iterate through each one and poll them, if they are not done
    /// add them to end of list. If they are done, remove them from list of tasks.
    fn poll_tasks(&mut self) {
        let count = self.tasks.len();
        for _ in 0..count {
            let task = self.tasks.remove(0);
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
                self.tasks.push(task);
            }
        }
    }
}

pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    Executor::spawn(future);
}
