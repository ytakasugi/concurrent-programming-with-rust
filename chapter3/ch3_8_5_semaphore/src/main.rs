mod semaphore;

use semaphore::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const NUM_LOOP: usize = 5;
const NUM_THREADS: usize = 2;
const SEM_NUM: isize = 1;

static mut CNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let mut v = Vec::new();
    let sem = Arc::new(Semaphore::new(SEM_NUM));

    for i in 0..NUM_THREADS {
        let s = sem.clone();
        let t = std::thread::spawn(move || {
            // スレッド毎にNUM_LOOP回ループする
            for _ in 0..NUM_LOOP {
                // セマフォ内のカウンタをインクリメント
                // もし、カウンタがSEM_NUM以上であれば、カウンタがSEM_NUM未満になるまで待機
                s.wait();
                // `fetch_add`で古い値を読み、それに加算した値を書き込み、古い値を返す
                unsafe { CNT.fetch_add(1, Ordering::SeqCst) };
                let n = unsafe { CNT.load(Ordering::SeqCst) };
                println!("{:?}, semaphore: i = {}, CNT = {}", std::thread::current().id(), i, n);
                assert!((n as isize) <= SEM_NUM);
                // `fetch_sub`で古い値を読み、それを減算した値を書き込み、古い値を返す
                unsafe { CNT.fetch_sub(1, Ordering::SeqCst) };
                // セマフォ内のカウンタをデクリメント
                s.post();
            }
        });
        v.push(t)
    }

    for t in v {
        t.join().unwrap();
    }
}
