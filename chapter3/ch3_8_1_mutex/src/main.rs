use std::sync::{Arc, Mutex};
use std::thread;

// Mutex: Read Lockは複数獲得可能、Write Lockは必ず1つのみ
// Arc: スレッドセーフな参照カウンタ型のスマートポインタ
fn some_func(lock: Arc<Mutex<u64>>) {
    loop {
        let mut val = lock.lock().unwrap();
        *val += 1;
        println!("{}", *val);
        // データがスコープを外れた際にロックが解除される
    }
}

fn main() {
    let lock0 = Arc::new(Mutex::new(0));
    // 参照(=強参照)カウンタをインクリメント。中身はクローンされない
    let lock1 = lock0.clone();

    // スレッドを生成
    let thread0 = thread::spawn(move || {
        some_func(lock0);
    });

    let thread1 = thread::spawn(move || {
        some_func(lock1);
    });

    thread0.join().unwrap();
    thread1.join().unwrap();
}
