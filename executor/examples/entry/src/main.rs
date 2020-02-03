use core::future::Future;

use core::{
    pin::Pin,
    task::{Context,Poll},
};

struct Foo{}

impl Future for Foo {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}


fn a() -> impl Future<Output = ()>{
    println!("hello world");
    Foo{}
}

#[executor::entry]
async fn foo(t:i32) {
    a(t).await;
}


#[executor::entry]
async fn main() {
    a().await;
}
