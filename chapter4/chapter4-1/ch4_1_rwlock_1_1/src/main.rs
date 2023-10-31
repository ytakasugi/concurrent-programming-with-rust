use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        // Readロックを獲得
        let flag = val.read().unwrap();
        if *flag {
            // Readロックを獲得中にWriteロックを獲得。
            // デッドロックが発生
            *val.write().unwrap() = false;
            println!("flag is true");
        }
    });

    t.join().unwrap();
}
