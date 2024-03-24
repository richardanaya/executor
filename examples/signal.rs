use std::{
    sync::{Arc, Mutex},
    task::{Context, Poll},
    thread::{self, sleep},
    time::Duration,
};

struct AsyncSignalWaiterState {
    ready: bool,
    waker: Option<core::task::Waker>,
}

struct AsyncSignalWaiter {
    state: Arc<Mutex<AsyncSignalWaiterState>>,
}

impl std::future::Future for AsyncSignalWaiter {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if !state.ready {
            state.waker = Some(cx.waker().clone());
            println!("pending...");
            Poll::Pending
        } else {
            println!("ready");
            Poll::Ready(())
        }
    }
}

// Lets image, that thread's body (exclude sleep)) is our mcu interrupt event.
fn receive_signal_from_somewhere(state: Arc<Mutex<AsyncSignalWaiterState>>) {
    thread::spawn(move || {
        sleep(Duration::from_millis(1000));
        let mut state = state.lock().unwrap();
        state.ready = true;
        state.waker.as_mut().unwrap().wake_by_ref();
    });
}

fn main() -> () {
    let state = Arc::new(Mutex::new(AsyncSignalWaiterState {
        ready: false,
        waker: None,
    }));
    executor::add_async(async move {
        let our_future = AsyncSignalWaiter {
            state: state.clone(),
        };
        receive_signal_from_somewhere(state.clone());
        our_future.await;
    });

    // slow timer updates all tasks, Dont need polls many tasks, that dont be ready soon or dont time sensitive.
    let update_event = thread::spawn(|| {
        while !executor::is_done() {
            executor::update();
            thread::sleep(Duration::from_millis(500));
        }
    });

    // fast timer updates waked tasks. There are not many of them, but they may be time sensitive.
    let woken_update_event = thread::spawn(|| {
        while !executor::is_done() {
            executor::update_woken();
            thread::sleep(Duration::from_millis(50));
        }
    });

    // only for properly shutdown.
    _ = update_event.join();
    _ = woken_update_event.join();
    println!("done")
}
