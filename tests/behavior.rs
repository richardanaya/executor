//! Behavior of the global executor.
//!
//! The executor is process-global, so every test holds `TEST_LOCK` and leaves
//! the queues empty. Run with `--test-threads=1` as well; the lock is what
//! actually keeps the tests from stepping on each other.

use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};
use std::cell::Cell;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_executor() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
    assert!(
        executor::is_done(),
        "executor was not empty; a previous test left tasks behind"
    );
    guard
}

fn drive(limit: usize) {
    for _ in 0..limit {
        if executor::is_done() {
            return;
        }
        executor::update();
    }
    assert!(
        executor::is_done(),
        "executor did not finish within {limit} updates"
    );
}

/// Run `drive` off the test thread so a non-reentrant spin lock cannot hang CI.
fn drive_within(limit: usize, timeout: Duration) {
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drive(limit)));
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => {}
        Ok(Err(payload)) => std::panic::resume_unwind(payload),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            panic!("update deadlocked or did not finish within {timeout:?}")
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            panic!("driver thread exited without a result")
        }
    }
}

struct Slot {
    ready: bool,
    waker: Option<Waker>,
}

struct Wait {
    slot: Arc<Mutex<Slot>>,
}

impl Future for Wait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut slot = self.slot.lock().unwrap();
        if slot.ready {
            Poll::Ready(())
        } else {
            slot.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn fresh_slot() -> Arc<Mutex<Slot>> {
    Arc::new(Mutex::new(Slot {
        ready: false,
        waker: None,
    }))
}

/// Poll once so the task can store its waker, and return that waker.
fn arm(slot: &Arc<Mutex<Slot>>) -> Waker {
    let task_slot = Arc::clone(slot);
    executor::add_async(async move {
        Wait { slot: task_slot }.await;
    });
    executor::update();
    // The task is pending, so the executor is not done yet.
    let waker = slot.lock().unwrap().waker.clone().expect("waker stored");
    waker
}

struct PollCount {
    polls: Arc<AtomicUsize>,
}

impl Future for PollCount {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(())
    }
}

struct Gate {
    polls: Arc<AtomicUsize>,
    go: Arc<AtomicBool>,
}

impl Future for Gate {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if self.go.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct Steps {
    left: u32,
    polls: Arc<AtomicUsize>,
}

impl Future for Steps {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if self.left == 0 {
            return Poll::Ready(());
        }
        self.left -= 1;
        cx.waker().wake_by_ref();
        if self.left == 0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct WakeAndSpawn {
    polls: u8,
    spawned: Arc<AtomicBool>,
}

impl Future for WakeAndSpawn {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.polls == 0 {
            self.polls = 1;
            cx.waker().wake_by_ref();
            let spawned = Arc::clone(&self.spawned);
            executor::add_async(async move {
                spawned.store(true, Ordering::SeqCst);
            });
            return Poll::Pending;
        }
        Poll::Ready(())
    }
}

struct DropCount(Arc<AtomicUsize>);

impl Drop for DropCount {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn empty_executor_is_done_and_update_is_a_noop() {
    let _guard = lock_executor();
    assert!(executor::is_done());
    executor::update();
    executor::update_woken();
    assert!(executor::is_done());
}

#[test]
fn ready_task_runs_and_is_done_becomes_true() {
    let _guard = lock_executor();
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    executor::add_async(async move {
        flag.store(true, Ordering::SeqCst);
    });
    assert!(!executor::is_done(), "queued task is not done yet");
    executor::update();
    assert!(ran.load(Ordering::SeqCst));
    assert!(executor::is_done());
}

#[test]
fn many_tasks_all_complete() {
    let _guard = lock_executor();
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..64 {
        let count = Arc::clone(&count);
        executor::add_async(async move {
            count.fetch_add(1, Ordering::SeqCst);
        });
    }
    assert!(!executor::is_done());
    drive(8);
    assert_eq!(count.load(Ordering::SeqCst), 64);
    assert!(executor::is_done());
}

#[test]
fn task_spawned_from_inside_a_task_is_picked_up() {
    let _guard = lock_executor();
    let phase = Arc::new(AtomicUsize::new(0));
    let parent = Arc::clone(&phase);
    let child = Arc::clone(&phase);
    let grand = Arc::clone(&phase);
    executor::add_async(async move {
        executor::add_async(async move {
            executor::add_async(async move {
                grand.fetch_add(1, Ordering::SeqCst);
            });
            child.fetch_add(1, Ordering::SeqCst);
        });
        parent.fetch_add(1, Ordering::SeqCst);
    });
    drive(8);
    assert_eq!(phase.load(Ordering::SeqCst), 3);
    assert!(executor::is_done());
}

fn wake_from_thread(waker: Waker, by_ref: bool) {
    thread::spawn(move || {
        if by_ref {
            waker.wake_by_ref();
        } else {
            waker.wake();
        }
    })
    .join()
    .unwrap();
}

#[test]
fn wake_by_ref_from_another_thread_completes_via_update_woken() {
    let _guard = lock_executor();
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let slot = fresh_slot();
    let slot_for_task = Arc::clone(&slot);
    executor::add_async(async move {
        Wait {
            slot: slot_for_task,
        }
        .await;
        flag.store(true, Ordering::SeqCst);
    });
    executor::update();
    let waker = slot.lock().unwrap().waker.clone().expect("waker stored");

    wake_from_thread(waker, true);
    slot.lock().unwrap().ready = true;
    executor::update_woken();
    assert!(
        ran.load(Ordering::SeqCst),
        "update_woken polls the woken task through to the code after await"
    );
    assert!(
        !executor::is_done(),
        "the finished task stays on the main list until update drops it"
    );
    executor::update();
    assert!(executor::is_done());
}

#[test]
fn consuming_wake_from_another_thread_completes() {
    let _guard = lock_executor();
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let slot = fresh_slot();
    let slot_for_task = Arc::clone(&slot);
    executor::add_async(async move {
        Wait {
            slot: slot_for_task,
        }
        .await;
        flag.store(true, Ordering::SeqCst);
    });
    executor::update();
    let waker = slot.lock().unwrap().waker.take().expect("waker stored");

    wake_from_thread(waker, false);
    slot.lock().unwrap().ready = true;
    executor::update_woken();
    assert!(ran.load(Ordering::SeqCst));
    assert!(!executor::is_done());
    executor::update();
    assert!(executor::is_done());
}

#[test]
fn waking_and_spawning_during_poll_does_not_deadlock() {
    let _guard = lock_executor();
    let spawned = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&spawned);
    executor::add_async(async move {
        WakeAndSpawn {
            polls: 0,
            spawned: flag,
        }
        .await;
    });

    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            executor::update();
            executor::update();
        }));
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => {}
        Ok(Err(payload)) => std::panic::resume_unwind(payload),
        Err(_) => panic!("update deadlocked while a task woke and spawned during poll"),
    }
    assert!(spawned.load(Ordering::SeqCst));
    assert!(executor::is_done());
}

#[test]
fn self_wake_during_poll_completes_without_deadlock() {
    let _guard = lock_executor();
    let polls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&polls);
    executor::add_async(async move {
        Steps {
            left: 4,
            polls: seen,
        }
        .await;
    });
    drive_within(32, Duration::from_secs(2));
    assert!(executor::is_done());
    assert!(
        polls.load(Ordering::SeqCst) >= 4,
        "each step should be polled"
    );
}

#[test]
fn ready_task_is_not_polled_again() {
    let _guard = lock_executor();
    let polls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&polls);
    executor::add_async(async move {
        PollCount { polls: seen }.await;
    });
    executor::update();
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    assert!(executor::is_done());
    executor::update();
    executor::update_woken();
    executor::update();
    assert_eq!(polls.load(Ordering::SeqCst), 1);
}

#[test]
fn woken_task_is_not_polled_after_ready() {
    let _guard = lock_executor();
    let polls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&polls);
    let slot = fresh_slot();
    let slot_for_task = Arc::clone(&slot);
    executor::add_async(async move {
        Wait {
            slot: slot_for_task,
        }
        .await;
        PollCount { polls: seen }.await;
    });
    executor::update();
    let waker = slot.lock().unwrap().waker.clone().expect("waker");
    slot.lock().unwrap().ready = true;
    waker.wake_by_ref();
    executor::update_woken();
    // Inner Wait returned Ready and the following PollCount ran in that same
    // poll of the async block. One poll of PollCount, then the task is done
    // but still listed.
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    executor::update();
    executor::update();
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    assert!(executor::is_done());
}

#[test]
fn update_woken_does_not_poll_unrelated_tasks() {
    let _guard = lock_executor();
    let polls = Arc::new(AtomicUsize::new(0));
    let go = Arc::new(AtomicBool::new(false));
    let polls_task = Arc::clone(&polls);
    let go_task = Arc::clone(&go);
    executor::add_async(async move {
        Gate {
            polls: polls_task,
            go: go_task,
        }
        .await;
    });
    let slot = fresh_slot();
    let waker = arm(&slot);
    // `arm` already polled every queued task once, including the gate.
    assert_eq!(polls.load(Ordering::SeqCst), 1);

    slot.lock().unwrap().ready = true;
    waker.wake_by_ref();
    executor::update_woken();
    assert_eq!(
        polls.load(Ordering::SeqCst),
        1,
        "update_woken must not poll a task that was not woken"
    );

    go.store(true, Ordering::SeqCst);
    executor::update();
    assert_eq!(polls.load(Ordering::SeqCst), 2);
    assert!(executor::is_done());
}

#[test]
fn is_done_tracks_input_queue_and_completed_tasks() {
    let _guard = lock_executor();
    assert!(executor::is_done());

    let slot = fresh_slot();
    let slot_for_task = Arc::clone(&slot);
    executor::add_async(async move {
        Wait {
            slot: slot_for_task,
        }
        .await;
    });
    assert!(
        !executor::is_done(),
        "a task sitting in the input queue is not done"
    );

    executor::update();
    assert!(!executor::is_done(), "pending task is not done");

    let waker = slot.lock().unwrap().waker.clone().unwrap();
    slot.lock().unwrap().ready = true;
    waker.wake();
    assert!(
        !executor::is_done(),
        "a woken task is still queued until it is polled and dropped"
    );

    executor::update_woken();
    assert!(
        !executor::is_done(),
        "update_woken finishes the task but leaves it listed until update"
    );
    executor::update();
    assert!(executor::is_done());
}

#[test]
fn non_unit_outputs_run() {
    let _guard = lock_executor();
    let number = Arc::new(AtomicUsize::new(0));
    let number_task = Arc::clone(&number);
    executor::add_async(async move {
        let value: u32 = async { 42u32 }.await;
        number_task.store(value as usize, Ordering::SeqCst);
        value
    });

    let text = Arc::new(Mutex::new(String::new()));
    let text_task = Arc::clone(&text);
    executor::add_async(async move {
        let value = String::from("executor");
        *text_task.lock().unwrap() = value.clone();
        value
    });

    // `Cell<u32>` is `Send` and not `Sync`. Output only has to be `Send`.
    executor::add_async(async { Cell::new(7u32) });

    drive(8);
    assert_eq!(number.load(Ordering::SeqCst), 42);
    assert_eq!(text.lock().unwrap().as_str(), "executor");
    assert!(executor::is_done());
}

struct Hold {
    drops: Arc<AtomicUsize>,
    slot: Arc<Mutex<Slot>>,
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl Future for Hold {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut slot = self.slot.lock().unwrap();
        if slot.ready {
            Poll::Ready(())
        } else {
            slot.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[test]
fn completed_tasks_are_dropped() {
    let _guard = lock_executor();
    let drops = Arc::new(AtomicUsize::new(0));
    let output_drops = Arc::clone(&drops);
    executor::add_async(async move { DropCount(output_drops) });
    executor::update();
    assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "Ready output is dropped when the task finishes"
    );
    assert!(executor::is_done());

    // `Hold`'s Drop runs when the future is dropped, not when poll returns Ready.
    let held = Arc::new(AtomicUsize::new(0));
    let slot = fresh_slot();
    executor::add_async(Hold {
        drops: Arc::clone(&held),
        slot: Arc::clone(&slot),
    });
    executor::update();
    assert_eq!(
        held.load(Ordering::SeqCst),
        0,
        "pending task is still alive"
    );

    let waker = slot.lock().unwrap().waker.take().unwrap();
    slot.lock().unwrap().ready = true;
    waker.wake_by_ref();
    executor::update_woken();
    assert_eq!(
        held.load(Ordering::SeqCst),
        0,
        "update_woken finishes the future but the main list still owns it"
    );
    drop(waker);
    assert_eq!(
        held.load(Ordering::SeqCst),
        0,
        "the waker was not the last owner; the executor list still is"
    );
    executor::update();
    assert_eq!(
        held.load(Ordering::SeqCst),
        1,
        "update drops the finished task"
    );
    assert!(executor::is_done());
}

#[test]
fn many_wakes_finish_quickly() {
    let _guard = lock_executor();
    let polls = Arc::new(AtomicUsize::new(0));
    const TASKS: usize = 8;
    const STEPS: u32 = 16;
    for _ in 0..TASKS {
        let polls = Arc::clone(&polls);
        executor::add_async(async move {
            Steps { left: STEPS, polls }.await;
        });
    }
    let started = Instant::now();
    // Miri interprets every poll, so the wall-clock budget is looser there.
    let budget = if cfg!(miri) { 60 } else { 2 };
    drive_within(10_000, Duration::from_secs(budget));
    if !cfg!(miri) {
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    assert!(executor::is_done());
    assert!(polls.load(Ordering::SeqCst) >= TASKS * STEPS as usize);
}
