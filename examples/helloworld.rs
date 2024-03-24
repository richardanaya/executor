use core::future::Future;

use core::{
    pin::Pin,
    task::{Context, Poll},
};

struct Foo {}

impl Future for Foo {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

async fn a() {
    println!("hello world");
    Foo {}.await;
    println!("goodbye world");
}

fn main() -> () {
    executor::add_async(async {
        a().await;
    });
    while !executor::is_done() {
        executor::update();
    }
}
