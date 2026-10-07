#![no_std]

use core::{future, sync::atomic::AtomicBool};
extern crate alloc;
use {
    alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc, task::Wake},
    core::{
        pin::Pin,
        sync::atomic::Ordering,
        task::{Context, Poll, Waker},
    },
    spin::{Lazy, Mutex},
};

type TasksList = VecDeque<Box<dyn Pendable + core::marker::Send + core::marker::Sync>>;
type Future<T> = Pin<Box<dyn future::Future<Output = T> + Send + 'static>>;

/// Executor struct type
struct Executor {
    /// Tasks collection. Use [`update()`] functon for polling all tasks and continue them progress.
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
    future: Mutex<Future<T>>,
    /// Returns `true` if the state is corresponding to [`core::task::Poll::Ready`] otherwise - false.
    ///
    /// Needed to determine, which task we shall drop.
    done: AtomicBool,
}

// What to do when a task's `Waker` is called.
// `Arc<Task<T>>` becomes a `Waker` through `alloc::task::Wake` and `Waker::from`.
// `wake_by_ref` clones the `Arc` and calls this.
impl<T: 'static> Wake for Task<T> {
    fn wake(self: Arc<Self>) {
        // Its tempting to call update() here, but dont:
        // 1) a wake can arrive long after the last poll (sleep, another thread).
        // 2) spin::Mutex is not reentrant. Polling while `update` / `update_woken`
        //    is already on the stack can lock this same queue. Those functions
        //    drain the queue and drop the guard before they poll.
        WOKEN_TASK_QUEUE.lock().push_back(Box::new(self));
    }
}

impl<T: 'static> Pendable for Arc<Task<T>> {
    fn update(&self) {
        if !self.future.is_locked() {
            let mut future = self.future.lock();
            let waker = Waker::from(self.clone());
            // Poll our future.
            // If future is done, mark it via Task<T>::done field.
            // We can't poll "done futures", so we mark "done futures" at Task<T>
            // and drop it at next run() call.
            let context = &mut Context::from_waker(&waker);
            self.done.store(
                !matches!(future.as_mut().poll(context), Poll::Pending),
                Ordering::Relaxed,
            );
        }
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

    /// Add task for a future to the list of tasks.  
    fn add_asyncs_from_buffer(&mut self) {
        let mut input_queue = INPUT_TASK_QUEUE.lock();
        while input_queue.len() > 0 {
            self.tasks.push_back(input_queue.pop_front().unwrap());
        }
    }

    /// Adds [`Future<T>`] to executor's tasks container.
    #[allow(dead_code)]
    fn add_async<T>(&mut self, future: Future<T>)
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
    /// When all tasks will done and we add new task and run them, old completed tasks will be removed from [`Executor::tasks`].
    fn update(&mut self) {
        self.update_woken();
        self.add_asyncs_from_buffer();
        for _ in 0..self.tasks.len() {
            let task = self.tasks.pop_front().unwrap();
            if task.is_done() {
                continue;
            }
            task.update();
            if !task.is_done() {
                self.tasks.push_back(task);
            }
        }
    }

    /// Polls tasks queued by a waker. Does not remove them from the main list.
    ///
    /// Completed tasks stay in [`Executor::tasks`] until [`Executor::update`] drops them.
    fn update_woken(&self) {
        poll_woken_queue();
    }
}

fn new_executor() -> Mutex<Executor> {
    Mutex::new(Executor {
        tasks: VecDeque::new(),
    })
}

// `VecDeque::new` is not const on the declared MSRV (1.63), so the queues are
// built on first use. `Lazy` is still `no_std`.
static DEFAULT_EXECUTOR: Lazy<Mutex<Executor>> = Lazy::new(new_executor);

fn new_task_queue() -> Mutex<TasksList> {
    Mutex::new(VecDeque::new())
}

/// Its tempts to add futuures to executor dirctly, without global container,
/// but if we will try add new future, during [`update()`],
/// will produse dead lock state, because [`update()`] already lock executor.
static INPUT_TASK_QUEUE: Lazy<Mutex<TasksList>> = Lazy::new(new_task_queue);

/// Its tempting to have this container internally,
/// but when we will add new tasks, for getting reference on this container,
/// we will have to lock executor,
/// that will dead lock our program if it perform during [`update()`] function, that aslo lock executor.
static WOKEN_TASK_QUEUE: Lazy<Mutex<TasksList>> = Lazy::new(new_task_queue);

/// Polls all pending tasks on global executor and remove completed tasks.
pub fn update() {
    DEFAULT_EXECUTOR.lock().update();
}

/// Polls all awaked tasks on global executor.
pub fn update_woken() {
    poll_woken_queue();
}

/// Drain the woken queue, release it, then poll.
///
/// The guard is dropped before any poll so a `wake` from inside a task can
/// enqueue again. `spin::Mutex` is not reentrant. Tasks queued by that wake
/// wait for the next `update_woken`. Tasks already marked done are not polled.
fn poll_woken_queue() {
    let batch = {
        let mut woken_tasks = WOKEN_TASK_QUEUE.lock();
        core::mem::take(&mut *woken_tasks)
    };
    for task in batch {
        if !task.is_done() {
            task.update();
        }
    }
}

/// Adds task for a future to the list of tasks.
pub fn add_async<T>(future: impl future::Future<Output = T> + 'static + Send)
where
    T: Send + 'static,
{
    let task = Arc::new(Task {
        future: Mutex::new(Box::pin(future)),
        done: AtomicBool::new(false),
    });

    INPUT_TASK_QUEUE.lock().push_back(Box::new(task));
}

/// Checks is uncompleted tasks remain.
pub fn is_done() -> bool {
    DEFAULT_EXECUTOR.lock().tasks.is_empty()
        && INPUT_TASK_QUEUE.lock().is_empty()
        && WOKEN_TASK_QUEUE.lock().is_empty()
}
