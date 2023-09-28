#![no_std]

use core::sync::atomic::AtomicBool;
extern crate alloc;
use {
    alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc},
    core::{
        future::Future,
        pin::Pin,
        sync::atomic::Ordering,
        task::{Context, Poll},
    },
    spin::Mutex,
    woke::{waker_ref, Woke},
};

type TasksList = VecDeque<Box<dyn Pendable + core::marker::Send + core::marker::Sync>>;

/// Executor struct type
struct Executor {
    /// Tasks conllection
    tasks: TasksList,
}

/// [`Task<T>`] interface for executor
///
/// [`Executor::tasks`] contain any [`Task`], that implement this interface.
trait Pendable {
    /// Updates future progress via calling [`Future::poll()`].
    ///
    /// Shall contain future's status internally and corresponding to [`Future::poll`] state.
    fn update(&self);
    /// Returns `true` if the state is corresponding to [`core::task::Poll::Ready`] otherwise - false.
    ///
    /// Needed to determine, which task we shall drop.
    fn is_done(&self) -> bool;
}

/// Container for [`Future`] and [`Future`]'s state, like [`Task::done`].
///
/// Task is our unit of execution and holds a future are waiting on.
struct Task<T> {
    future: Mutex<Pin<Box<dyn Future<Output = T> + Send + 'static>>>,
    done: AtomicBool,
}

// Implement what we would like to do when a task gets woken up.
impl<T> Woke for Task<T> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.update();
    }
}

impl<T> Pendable for Arc<Task<T>> {
    fn update(&self) {
        let mut future = self.future.lock();
        let waker = waker_ref(self);
        // Poll our future.
        // If future is done, mark it via Task<T>::done field.
        // We can't poll "done futures", so we mark "done futures" at Task<T>
        // and drop it at next run() or gerbage_collect() call.
        let context = &mut Context::from_waker(&waker);
        self.done.store(
            !matches!(future.as_mut().poll(context), Poll::Pending),
            Ordering::Relaxed,
        );
    }

    fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }
}

impl Executor {
    /// Adds [`Task<T>`] to executor's tasks container.
    fn add_task<T>(&mut self, task: Arc<Task<T>>)
    where
        T: Send + 'static,
    {
        self.tasks.push_back(Box::new(task));
    }

    /// Adds [`Task<T>`] to executor's tasks container and immediately poll it.
    fn run_n_add_async<T>(&mut self, future: Pin<Box<dyn Future<Output = T> + 'static + Send>>)
    where
        T: Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(future),
            done: AtomicBool::new(false),
        });
        task.update();
        self.add_task(task);
    }

    /// Add task for a future to the list of tasks.
    fn add_async<T>(&mut self, future: Pin<Box<dyn Future<Output = T> + 'static + Send>>)
    where
        T: Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(future),
            done: AtomicBool::new(false),
        });
        self.add_task(task);
    }

    /// Polls all pending tasks on global executor and remove completed tasks.
    ///
    /// You may notice, that when all tasks will done, we keep them, although this is objectively useless.
    /// I think, finding our each complited task from [`Woke::wake_by_ref()`] at [`Executor::tasks`] and drop it is too expensive,
    /// so we just mark them via [`Task<T>::done`] field as done.
    /// When all tasks will done and we add new task and run them, old completed tasks will be removed from [`Executor::tasks`] or [`Executor::gerbage_collect()`].
    fn run(&mut self) {
        for _ in 0..self.tasks.len() {
            let task = self.tasks.pop_front().unwrap();
            if !task.is_done() {
                task.update();
                self.tasks.push_back(task);
            }
        }
    }

    /// Removes completed task from [`Executor::tasks`].
    ///
    /// As you may also notice, same as [`Executor::run()`], but don't poll tasks.
    /// Only drop completed tasks.
    fn gerbage_collect(&mut self) {
        for _ in 0..self.tasks.len() {
            let task = self.tasks.pop_front().unwrap();
            if !task.is_done() {
                self.tasks.push_back(task);
            }
        }
    }
}

static DEFAULT_EXECUTOR: Mutex<Executor> = Mutex::new(Executor {
    tasks: VecDeque::new(),
});

/// Polls all pending tasks on global executor and remove completed tasks.
pub fn run() {
    DEFAULT_EXECUTOR.lock().run();
}

/// Adds task for a future to the list of tasks.
pub fn add_async<T>(future: impl Future<Output = T> + 'static + Send)
where
    T: Send + 'static,
{
    DEFAULT_EXECUTOR.lock().add_async(Box::pin(future));
}

/// Drops completed tasks and checks is uncompleted tasks remain.
pub fn is_done() -> bool {
    let mut exec = DEFAULT_EXECUTOR.lock();
    exec.gerbage_collect();
    exec.tasks.is_empty()
}

/// Adds task for a future to the list of tasks.
pub fn run_n_add_async<T>(future: impl Future<Output = T> + 'static + Send)
where
    T: Send + 'static,
{
    DEFAULT_EXECUTOR.lock().run_n_add_async(Box::pin(future));
}
