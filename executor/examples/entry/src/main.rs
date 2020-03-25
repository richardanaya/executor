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
    println!("a");
    Foo {}
}

async fn b() {
    println!("b");
    a().await;
}

async fn c() {
    println!("c");
    b().await;
    b().await;
}

#[executor::entry]
pub async fn main() {
    c().await;
}
