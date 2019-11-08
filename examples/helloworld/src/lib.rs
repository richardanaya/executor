#![no_std]
#![feature(alloc_error_handler)]
#![feature(lang_items)]

use core::future::Future;

use executor::*;
use core::{
    pin::Pin,
    task::{Context,Poll},
};

#[global_allocator]
static ALLOCATOR:malloc::Allocator = malloc::Allocator;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn oom(_: core::alloc::Layout) -> ! {
    loop {}
}

struct Foo{}

impl Future for Foo {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

extern {
    fn test(i:i32);
}

fn a() -> impl Future<Output = ()>{
    unsafe { test(1); }
    Foo{}
}

#[no_mangle]
pub fn main() -> () {
    Executor::spawn(a());
}



#[lang = "eh_personality"] extern fn eh_personality() {}