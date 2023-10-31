use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        // Readロックを獲得。スコープを抜けるまでロックを獲得したまま。
        let _flag = val.read().unwrap();
        // Readロックを獲得中にWriteロックを獲得。
        // デッドロックが発生
        *val.write().unwrap() = false;
        println!("deadlock");
    });

    t.join().unwrap();
}