use core::future::Future;

use executor::*;
use core::{
    pin::Pin,
    task::{Context,Poll},
};

struct Foo{}

impl Future for Foo {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}


fn a() -> impl Future<Output = ()>{
    println!("hello world");
    Foo{}
}

fn main() -> () {
    Executor::spawn(a());
}
