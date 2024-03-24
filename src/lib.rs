#![no_std]

use core::{future, sync::atomic::AtomicBool};
extern crate alloc;
use {
    alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc},
    core::{
        pin::Pin,
        sync::atomic::Ordering,
        task::{Context, Poll},
    },
    spin::Mutex,
    woke::{waker_ref, Woke},
};

type TasksList = VecDeque<Box<dyn Pendable + core::marker::Send + core::marker::Sync>>;
type Future<T> = Pin<Box<dyn future::Future<Output = T> + Send + 'static>>;

/// Executor struct type
struct Executor {
    /// Tasks collection. Use [`update()`] functon for polling all tasks and continue them progress.
    tasks: TasksList,

    /// Woken Tasks collection. Contain woken tasks from [`Executor::tasks`].
    /// [`update()`] function aslo can continue progress of this tasks,
    /// but [`update_woken()`] function will be preferred, because poll only tasks, that was awakened.
    woken_tasks: &'static Mutex<TasksList>,
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
    /// Awaked tasks collection.
    ///
    /// Shall contain itself to this, when was awaked by ['wake() or wake_by_ref()'].
    woken_tasks: &'static Mutex<TasksList>,
    future: Mutex<Future<T>>,
    /// Returns `true` if the state is corresponding to [`core::task::Poll::Ready`] otherwise - false.
    ///
    /// Needed to determine, which task we shall drop.
    done: AtomicBool,
}

// Implement what we would like to do when a task gets woken up.
impl<T: 'static> Woke for Task<T> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let woken = arc_self.clone();
        // its tempting to call update() "in place", but dont do this for 2 reason:
        // 1) for some reason, somethimes cant poll our future, exactly after big latency between update's(), for example, because of sleep() call.
        // 2) if call wake() or wake_by_ref() at poll, being at woken_tasks, will produce dead lock state.
        arc_self.woken_tasks.lock().push_back(Box::new(woken));
    }
}

impl<T: 'static> Pendable for Arc<Task<T>> {
    fn update(&self) {
        if !self.future.is_locked() {
            let mut future = self.future.lock();
            let waker = waker_ref(self);
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
            woken_tasks: &WOKEN_TASK_QUEUE,
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
            if !task.is_done() {
                task.update();
                self.tasks.push_back(task);
            }
        }
    }

    /// Polls all pending tasks on global executor and remove completed tasks.
    ///
    /// When all tasks will done and we add new task and run them, old completed tasks will be removed from [`Executor::tasks`].
    fn update_woken(&self) {
        let mut woken_tasks = self.woken_tasks.lock();
        while !woken_tasks.is_empty() {
            woken_tasks.pop_front().unwrap().as_mut().update();
        }
    }
}

static DEFAULT_EXECUTOR: Mutex<Executor> = Mutex::new(Executor {
    woken_tasks: &WOKEN_TASK_QUEUE,
    tasks: VecDeque::new(),
});

/// Its tempts to add futuures to executor dirctly, without global container,
/// but if we will try add new future, during [`update()`],
/// will produse dead lock state, because [`update()`] already lock executor.
static INPUT_TASK_QUEUE: Mutex<TasksList> = Mutex::new(VecDeque::new());

/// Its tempting to have this container internally,
/// but when we will add new tasks, for getting reference on this container,
/// we will have to lock executor,
/// that will dead lock our program if it perform during [`update()`] function, that aslo lock executor.
static WOKEN_TASK_QUEUE: Mutex<TasksList> = Mutex::new(VecDeque::new());

/// Polls all pending tasks on global executor and remove completed tasks.
pub fn update() {
    DEFAULT_EXECUTOR.lock().update();
}

/// Polls all awaked tasks on global executor.
pub fn update_woken() {
    let mut woken_tasks = WOKEN_TASK_QUEUE.lock();
    while !woken_tasks.is_empty() {
        woken_tasks.pop_front().unwrap().as_mut().update();
    }
}

/// Adds task for a future to the list of tasks.
pub fn add_async<T>(future: impl future::Future<Output = T> + 'static + Send)
where
    T: Send + 'static,
{
    let task = Arc::new(Task {
        woken_tasks: &WOKEN_TASK_QUEUE,
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
