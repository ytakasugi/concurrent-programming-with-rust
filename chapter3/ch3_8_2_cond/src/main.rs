use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn child(id: u64, p: Arc<(Mutex<bool>, Condvar)>) {
    // *pはArcを通じて(Mutex<bool>, Condvar)の所有権を獲得する
    // &*pは*pへの参照を取得する。
    // &(ref lock, ref cvar)は、この参照を通じてlockとcvarへの新しい束縛（変数）を作成します。
    // この際、refを使ってlockとcvarへの参照を作成する。
    // これにより、lockとcvarは元のタプル内のMutex<bool>とCondvarへの参照として扱える。
    // この手法は、元のタプル内のデータを変更しないで、lockとcvarにアクセスするために使用される。
    // MutexとCondvarは元のタプル内のデータを安全に保護し、refを使用することでそれらへの参照を明示的に作成する
    let &(ref lock, ref cvar) = &*p;

    // ロックを獲得する
    let started = lock.lock().unwrap();
    // 条件変数が通知を受け取るまで待機(共有変数がfalseの間待機する)
    let _c = cvar.wait_while(started, |started| !*started).unwrap();
    println!("child {}", id);
}

fn parent(p: Arc<(Mutex<bool>, Condvar)>) {
    let &(ref lock, ref cvar) = &*p;

    // Mutex Lockを行う
    let mut started = lock.lock().unwrap();
    // 共有変数を更新
    *started = true;
    cvar.notify_all();
    println!("parent");
}

fn main() {
    // ミューテックスと条件変数を作成
    let pair0 = Arc::new((Mutex::new(false), Condvar::new()));
    let pair1 = pair0.clone();
    let pair2 = pair0.clone();

    let c0 = thread::spawn(move || { child(0, pair0) });
    let c1 = thread::spawn(move || { child(1, pair1) });
    let p  = thread::spawn(move || { parent(pair2) });

    c0.join().unwrap();
    c1.join().unwrap();
    p.join().unwrap();
}
