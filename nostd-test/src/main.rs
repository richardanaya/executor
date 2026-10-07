//! Runs without `std`: global allocator, `add_async` / `update` / `is_done`.
//!
//! Build and run under QEMU:
//! `cargo build --manifest-path nostd-test/Cargo.toml --release --target thumbv7m-none-eabi`
//! `qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic -semihosting-config enable=on,target=native -kernel nostd-test/target/thumbv7m-none-eabi/release/executor-nostd-test`

#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::future::Future;
use core::panic::PanicInfo;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

const HEAP_SIZE: usize = 16 * 1024;

#[repr(C, align(16))]
struct Heap(core::cell::UnsafeCell<[u8; HEAP_SIZE]>);

unsafe impl Sync for Heap {}

static HEAP: Heap = Heap(core::cell::UnsafeCell::new([0; HEAP_SIZE]));
static HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

struct Bump;

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(1);
        let size = layout.size();
        let base = HEAP.0.get() as usize;
        loop {
            let current = HEAP_NEXT.load(Ordering::Relaxed);
            let start = (current + align - 1) & !(align - 1);
            let end = start.saturating_add(size);
            if end > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            if HEAP_NEXT
                .compare_exchange(current, end, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return (base + start) as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: Bump = Bump;

static DROPS: AtomicUsize = AtomicUsize::new(0);
static HITS: AtomicUsize = AtomicUsize::new(0);

struct Bomb;

impl Drop for Bomb {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

struct SelfWake {
    polls: u8,
}

impl Future for SelfWake {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.polls = self.polls.wrapping_add(1);
        if self.polls == 1 {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

extern "C" {
    static mut _sbss: u8;
    static mut _ebss: u8;
    static mut _sdata: u8;
    static mut _edata: u8;
    static _sidata: u8;
}

/// Hardware reset entry.
///
/// # Safety
///
/// The CPU must enter here once, with flash and RAM mapped as in `link.ld`,
/// before any other Rust code runs.
#[no_mangle]
pub unsafe extern "C" fn Reset() -> ! {
    init_ram();
    if run() {
        write0("executor nostd tests ok\n");
        exit(0);
    } else {
        write0("executor nostd tests FAILED\n");
        exit(1);
    }
}

unsafe fn init_ram() {
    let mut bss = core::ptr::addr_of_mut!(_sbss);
    let bss_end = core::ptr::addr_of_mut!(_ebss);
    while bss < bss_end {
        bss.write_volatile(0);
        bss = bss.add(1);
    }

    let mut data = core::ptr::addr_of_mut!(_sdata);
    let data_end = core::ptr::addr_of_mut!(_edata);
    let mut src = core::ptr::addr_of!(_sidata);
    while data < data_end {
        data.write_volatile(src.read_volatile());
        data = data.add(1);
        src = src.add(1);
    }
}

fn run() -> bool {
    if !executor::is_done() {
        return false;
    }

    let before = DROPS.load(Ordering::Relaxed);
    executor::add_async(async {
        let _bomb = Bomb;
    });
    if executor::is_done() {
        return false;
    }
    executor::update();
    if !executor::is_done() || DROPS.load(Ordering::Relaxed) != before + 1 {
        return false;
    }

    executor::add_async(async {
        executor::add_async(async {
            HITS.fetch_add(1, Ordering::Relaxed);
        });
        HITS.fetch_add(1, Ordering::Relaxed);
    });
    drive(4);
    if HITS.load(Ordering::Relaxed) != 2 || !executor::is_done() {
        return false;
    }

    executor::add_async(SelfWake { polls: 0 });
    drive(4);
    if !executor::is_done() {
        return false;
    }

    HITS.store(0, Ordering::Relaxed);
    for _ in 0..8 {
        executor::add_async(async {
            HITS.fetch_add(1, Ordering::Relaxed);
        });
    }
    drive(4);
    if HITS.load(Ordering::Relaxed) != 8 || !executor::is_done() {
        return false;
    }

    HITS.store(0, Ordering::Relaxed);
    executor::add_async(async {
        let value: u32 = async { 42u32 }.await;
        HITS.store(value as usize, Ordering::Relaxed);
        value
    });
    drive(4);
    HITS.load(Ordering::Relaxed) == 42 && executor::is_done()
}

fn drive(limit: usize) {
    for _ in 0..limit {
        if executor::is_done() {
            return;
        }
        executor::update();
    }
}

fn write0(text: &str) {
    let mut buf = [0u8; 80];
    let len = text.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&text.as_bytes()[..len]);
    unsafe {
        semihost(0x04, buf.as_ptr());
    }
}

fn exit(status: u32) -> ! {
    let block = [0x20026u32, status];
    loop {
        unsafe {
            semihost(0x20, block.as_ptr().cast());
        }
    }
}

unsafe fn semihost(op: u32, arg: *const u8) {
    core::arch::asm!(
        "bkpt #0xAB",
        in("r0") op,
        in("r1") arg,
        options(nostack)
    );
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write0("panic\n");
    exit(1);
}

unsafe extern "C" fn fault() -> ! {
    write0("fault\n");
    exit(1);
}

#[link_section = ".vector_table.reset"]
#[no_mangle]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;

#[link_section = ".vector_table.fault"]
#[no_mangle]
pub static FAULT_VECTORS: [unsafe extern "C" fn() -> !; 2] = [fault, fault];
