use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{fence, AtomicUsize, Ordering};

pub struct TicketLock<T> {
    ticket: AtomicUsize,
    turn: AtomicUsize,
    data: UnsafeCell<T>,
}

pub struct TicketLockGuard<'a, T> {
    ticket_lock: &'a TicketLock<T>,
}

impl<T> TicketLock<T> {
    pub fn new(v: T) -> Self {
        TicketLock {
            ticket: AtomicUsize::new(0),
            turn: AtomicUsize::new(0),
            data: UnsafeCell::new(v),
        }
    }

    pub fn lock(&self) -> TicketLockGuard<T> {
        // チケットを取得
        let t = self.ticket.fetch_add(1, Ordering::Relaxed);
        // 所有するチケットの順番になるまでスピン
        while self.turn.load(Ordering::Relaxed) != t {}
        fence(Ordering::Acquire);

        TicketLockGuard { ticket_lock: self }
    }
}

impl<'a, T> Drop for TicketLockGuard<'a, T> {
    fn drop(&mut self) {
        self.ticket_lock.turn.fetch_add(1, Ordering::Release);
    }
}

// TicketLock型はスレッド間で共有可能と設定
unsafe impl<T> Sync for TicketLock<T> {}
unsafe impl<T> Send for TicketLock<T> {}

impl<'a, T> Deref for TicketLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ticket_lock.data.get() }
    }
}

impl<'a, T> DerefMut for TicketLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ticket_lock.data.get() }
    }
}
