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

fn a() -> impl Future<Output = ()> {
    println!("hello world");
    Foo {}
}

async fn blah() {
    a().await;
}

fn main() -> () {
    executor::block_on(blah());
}
