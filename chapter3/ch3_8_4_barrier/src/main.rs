use std::sync::{Arc, Barrier};
use std::thread;

fn main() {
    // スレッドハンドラを保存するベクタ
    let mut v = Vec::new();
    // 10スレッド分のバリア同期をArcで包む
    let barrier = Arc::new(Barrier::new(10));

    for _ in 0..10 {
        let b = barrier.clone();
        let th = thread::spawn(move || {
            // すべてのスレッドが合流するまでスレッドをブロックする
            b.wait();
            println!("finished barrier");
        });
        v.push(th);
    }
    for th in v {
        th.join().unwrap();
    }
}
