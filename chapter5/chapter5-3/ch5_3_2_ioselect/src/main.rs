use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use nix::{
    errno::Errno,
    sys::{
        epoll::{
            epoll_create1, epoll_ctl, epoll_wait,
            EpollCreateFlags, EpollEvent, EpollFlags, EpollOp,
        },
        eventfd::{eventfd, EfdFlags},
    },
    unistd::{read, write}, libc::epoll_create1,
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
    let val = unsafe {
        std::slice::from_raw_parts(
            prt, std::mem::size_of_val(&n)
        )
    };
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
            epfd: epoll_create1(EpollCreateFlags::empty()).unwrap(),
            // eventfd生成
            event: eventfd(0, EfdFlags::empty()).unwrap(),
        };
        let result = Arc::new(s);
        let s = result.clone();
        // epoll用スレッドを生成
        std::thread::spawn(move || s.select());

        result
    }
}


fn main() {
    println!("Hello, world!");
}
