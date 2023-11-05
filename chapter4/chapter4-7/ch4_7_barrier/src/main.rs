use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const NUM_THREADS: usize = 4;
const NUM_LOOP: usize = 100000;

struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

// ロックの解放および、ロック中に保護対象データを操作するための型
struct SpinLockGuard<'a, T> {
    spin_lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    fn new(v: T) -> Self {
        SpinLock {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(v),
        }
    }

    fn lock(&self) -> SpinLockGuard<T> {
        loop {
            while self.lock.load(Ordering::Relaxed) {}

            if let Ok(_) =
                self.lock
                    // Atomic変数と第一引数が同じであるか確認し、同じであれば第二引数の値を第三引数のメモリ順序を指定してAtomic変数に設定する。
                    .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            {
                break;
            }
        }
        SpinLockGuard { spin_lock: self }
    }
}

// SpinLock型はスレッド間で共有可能と指定
unsafe impl<T> Sync for SpinLock<T> {}
unsafe impl<T> Send for SpinLock<T> {}

// ロック獲得後に自動で解放されるようにDropトレイトを実装
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.spin_lock.lock.store(false, Ordering::Release);
    }
}

// 保護対象データのimmutableな参照外し
impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.spin_lock.data.get() }
    }
}

// 保護対象データのmutableな参照外し
impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.spin_lock.data.get() }
    }
}

fn main() {
    let lock = Arc::new(SpinLock::new(0));
    let mut v = Vec::new();

    for _ in 0..NUM_THREADS {
        let lock0 = lock.clone();
        let t = std::thread::spawn(move || {
            for _ in 0..NUM_LOOP {
                let mut data = lock0.lock();
                *data += 1;
            }
        });
        v.push(t);
    }
    for t in v {
        t.join().unwrap();
    }

    println!(
        "COUNT = {} (expected = {})",
        *lock.lock(),
        NUM_LOOP * NUM_THREADS
    );
}
