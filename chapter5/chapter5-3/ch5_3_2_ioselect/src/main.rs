use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use nix::{
    errno::Errno,
    libc::epoll_create1,
    sys::{
        epoll::{
            epoll_create1, epoll_ctl, epoll_wait, EpollCreateFlags, EpollEvent, EpollFlags, EpollOp,
        },
        eventfd::{eventfd, EfdFlags},
    },
    unistd::{read, write},
};
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::io::{AsRawFd, RawFd},
    pin::Pin,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};

fn write_eventfd(fd: RawFd, n: usize) {
    // usizeを*const u8に変換
    let ptr = &n as *const usize as *const u8;
    // ポインタと長さからスライスを作成する
    let val = unsafe { std::slice::from_raw_parts(prt, std::mem::size_of_val(&n)) };
    // writeシステムコール呼び出し
    write(fd, &val).unwrap();
}

enum IOOps {
    // epollへ追加
    ADD(EpollCreateFlags, RawFd, Waker),
    // epollから削除
    REMOVE(RawFd),
}

struct IOSelector {
    // fdからwaker
    wakers: Mutex<HashMap<RawFd, Waker>>,
    // IOのキュー
    queue: Mutex<VecDeque<IOOps>>,
    // epollのfd
    epfd: RawFd,
    // eventfdのfd
    event: RawFd,
}

impl IOSelector {
    fn new() -> Arc<Self> {
        let s = IOSelector {
            wakers: Mutex::new(HashMap::new()),
            queue: Mutex::new(VecDeque::new()),
            epfd: unsafe { epoll_create1(EpollCreateFlags::empty()).unwrap() },
            // eventfd生成
            event: eventfd(0, EfdFlags::empty()).unwrap(),
        };
        let result = Arc::new(s);
        let s = result.clone();
        // epoll用スレッドを生成
        std::thread::spawn(move || s.select());

        result
    }

    // epollで監視するための関数
    fn add_event(
        &self,
        flag: EpollFlags,
        fd: RawFd,
        waker: Waker,
        wakers: &mut HashMap<RawFd, Waker>,
    ) {
        let epoll_add = EpollOp::EpollCtlAdd;
        let epoll_mod = EpollOp::EpollCtlMod;
        let epoll_one = EpollFlags::EPOLLONESHOT;

        // EPOLLONESHOTを指定して、一度イベントが発生すると
        // そのfdへのイベントは再設定するまで通知されないようになる
        let mut ev = EpollEvent::new(flag | epoll_one, fd as u64);

        // 監視対象に追加
        if let Err(err) = epoll_ctl(self.epfd, epoll_add, fd, &mut ev) {
            match err {
                nix::Error::Sys(Errno::EEXIST) => {
                    // 既に追加されていた場合は再設定
                    epoll_ctl(self.epfd, epoll_mod, fd, &mut ev).unwrap();
                }
                _ => {
                    panic!("epoll_ctd: {}", err);
                }
            }
        }
    }

    // epollの監視から削除するための関数
    fn rm_event(&self, fd: RawFd, wakers: &mut HashMap<RawFd, Waker>) {
        let epoll_del = EpollOp::EpollCtlDel;
        let mut ev = EpollEvent::new(EpollFlags::empty(), fd as u64);

        epoll_ctl(self.epfd, epoll_del, fd, &mut ev).ok();
        wakers.remove(&fd);
    }

    fn select(&self) {
        let epoll_in = EpollFlags::EPOLLIN;
        let epoll_add = EpollOp::EpollCtlAdd;

        // eventfdをepollの監視対象に追加
        let mut ev = EpollEvent::new(epoll_in, self.event as u64);
        epoll_ctl(self.epfd, epoll_add, self.event, &mut ev).unwrap();

        let mut events = vec![EpollEvent::empty(); 1024];
        // eventの発生を監視
        while let Ok(nfds) = epoll_wait(self.epfd, &mut events, -1) {
            let mut t = self.wakers.lock().unwrap();
            for n in 0..nfds {
                if events[n].data() == self.event as u64 {
                    // eventfdの場合、追加、削除要求を処理
                    let mut q = self.queue.lock().unwrap();

                    while let Some(op) = q.pop_front() {
                        match op {
                            // 追加
                            IOOps::ADD(flag, fd, waker) => 
                                self.add_event(flag, fd, waker, &mut t),
                            // 削除
                            IOOps::REMOVE(fd) => self.rm_event(fd, &mut t),
                        }
                    }
                    let mut buf: [u8; 8] = [0; 8];
                    // eventfdの通知削除
                    read(self.event, &mut buf).unwrap();
                } else {
                    // 実行キューに追加
                    let data = events[n].data() as i32;
                    let waker = t.remove(&data).unwrap();
                    waker.wake_by_ref();
                }
            }
        }
    }

    // ファイルディスクリプタ登録用関数
    fn register(&self, flags: EpollFlags, fd: RawFd, waker: Waker) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(IOOps::ADD(flags, fd, waker));
        write_eventfd(self.event, 1);
    }

    // ファイルディスクリプタ削除用関数
    fn unregister(&self, fd: RawFd) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(IOOps::REMOVE(fd));
        write_eventfd(self.event, 1);
    }

}

fn main() {
    println!("Hello, world!");
}
