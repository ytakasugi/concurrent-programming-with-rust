use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn child(id: u64, p: Arc<(Mutex<bool>, Condvar)>) {
    let &(ref lock, ref cvar) = &*p;

    // ミューテックスロックを行う
    let mut started = lock.lock().unwrap();
    // Mutex中の共有変数が`false`の間ループ
    while !*started {
        // `wait`で待機
        started = cvar.wait(started).unwrap();
    }
    println!("child {}", id);
}

fn parent(p: Arc<(Mutex<bool>, Condvar)>) {
    let &(ref lock, ref cvar) = &*p;

    let mut started = lock.lock().unwrap();
    // 共有変数を更新
    *started = true;
    // 通知
    cvar.notify_all();
    println!("parent");
}

fn main() {
    // Mutexと条件変数を作成
    let pair0 = Arc::new((Mutex::new(false), Condvar::new()));
    let pair1 = pair0.clone();
    let pair2 = pair0.clone();

    let c0 = thread::spawn(move || {child(0, pair0)});
    let c1 = thread::spawn(move || {child(1, pair1)});
    let c2 = thread::spawn(move || {parent(pair2)});

    c0.join().unwrap();
    c1.join().unwrap();
    c2.join().unwrap();
}
