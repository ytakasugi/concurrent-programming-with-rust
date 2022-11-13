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
    Hello,
    Wolrd,
    End,
}

impl Hello {
    fn new() -> Self {
        Hello {
            state: StateHello::Hello,
        }
    }
}

impl Future for Hello {
    type Output = ();

    // 実行関数 <3>
    fn poll(mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>) -> Poll<()> {
        match (*self).state {
            StateHello::Hello => {
                print!("Hello, ");
                // WORLD状態に遷移
                (*self).state = StateHello::Wolrd;
                Poll::Pending // 再度呼び出し可能
            }
            StateHello::Wolrd => {
                println!("World!");
                // END状態に遷移
                (*self).state = StateHello::End;
                Poll::Pending // 再度呼び出し可能
            }
            StateHello::End => {
                Poll::Ready(()) // 終了
            }
        }
    }
}

// 実行単位 <1>
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
    // 初期化
    let task = Arc::new(Task::new());
    let waker = waker_ref(&task);
    let mut ctx = Context::from_waker(&waker); // <2>
    let mut hello = task.hello.lock().unwrap();

    // 停止と再開の繰り返し <3>
    hello.as_mut().poll(&mut ctx);
    hello.as_mut().poll(&mut ctx);
    hello.as_mut().poll(&mut ctx);
}
