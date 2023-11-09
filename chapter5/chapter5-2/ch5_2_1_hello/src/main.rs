use futures::future::{BoxFuture, FutureExt};
use futures::task::{waker_ref, ArcWake};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

struct Hello {
    state: StateHello,
}

enum StateHello {
    HELLO,
    WORLD,
    END,
}

impl Hello {
    fn new() -> Self {
        Hello {
            // 初期状態
            state: StateHello::HELLO,
        }
    }
}

impl Future for Hello {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        match  (*self).state {
            StateHello::HELLO => {
                print!("Hello,");
                (*self).state = StateHello::WORLD;
                Poll::Pending
            }
            StateHello::WORLD => {
                print!("World!");
                (*self).state = StateHello::END;
                Poll::Pending
            }
            StateHello::END => {
                Poll::Ready(())
            }
        }
    }
}

struct Task {
    hello: Mutex<BoxFuture<'static, ()>>,
}

impl Task {
    fn new() -> Self {
        let hello = Hello::new();
        Task {
            hello: Mutex::new(hello.boxed()),
        }
    }
}

// 何もしない
impl ArcWake for Task {
    fn wake_by_ref(_arc_self: &Arc<Self>) {}
}

fn main() {
    let task = Arc::new(Task::new());
    let waker = waker_ref(&task);
    let mut ctx = Context::from_waker(&waker);
    let mut hello = task.hello.lock().unwrap();

    // 停止と再開
    let _ = hello.as_mut().poll(&mut ctx);
    let _ = hello.as_mut().poll(&mut ctx);
    let _ = hello.as_mut().poll(&mut ctx);
}
