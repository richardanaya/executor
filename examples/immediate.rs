use std::task::{Context, Poll};

struct ImmediatelyWakingFuture {
    first_call: bool,
}

impl std::future::Future for ImmediatelyWakingFuture {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.first_call {
            self.first_call = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

fn main() -> () {
    executor::run(async {
        ImmediatelyWakingFuture { first_call: false }.await;
        println!("hello")
    });
}
