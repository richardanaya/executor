use once_cell::sync::OnceCell;
use {
    std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    },
    woke::{waker_ref, Woke},
};

// our executor just holds one task
pub struct Executor {
    task: Option<Arc<Task>>,
}

// Our task holds onto a future the executor can poll
struct Task {
    pub future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
}

// specify how we want our tasks to wake up
impl Woke for Task {
    fn wake_by_ref(_: &Arc<Self>) {
        // run the executor again because something finished!
        Executor::run()
    }
}

impl Executor {
    pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
        // store our task in global state
        let task = Arc::new(Task {
            future: Mutex::new(Some(Box::pin(future))),
        });
        let mut e = get_executor().lock().unwrap();
        e.task = Some(task);

        // we drop this early because otherwise run() will cause a mutex lock
        std::mem::drop(e);

        // get things going!
        Executor::run();
    }
    fn run() {
        // get our task from global state
        let e = get_executor().lock().unwrap();
        if let Some(task) = &e.task {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // make a waker for our task
                let waker = waker_ref(&task);
                // poll our future and give it a waker
                let context = &mut Context::from_waker(&*waker);
                if let Poll::Pending = future.as_mut().poll(context) {
                    *future_slot = Some(future);
                }
            }
        }
    }
}

// get a global holder of our one task
fn get_executor() -> &'static Mutex<Executor> {
    static INSTANCE: OnceCell<Mutex<Executor>> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(Executor { task: None }))
}
