use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let val = Arc::new(RwLock::new(true));

    let t = thread::spawn(move || {
        // Readロックを獲得するが、即座に解放される
        drop(val.read().unwrap());
        // 以下のコードだと実行時エラーになるので、上記のようにdrop関数を使用する
        // let _ = val.read().unwrap();
        *val.write().unwrap() = false;
        println!("not deadlock");
    });

    t.join().unwrap();
}