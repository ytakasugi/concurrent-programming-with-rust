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
        eventfd::{eventfd, EfdFlags}, // eventfd用のインポート <1>
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
    let val = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of_val(&n)) };
    // writeシステムコール呼び出し
    write(fd, &val).unwrap();
}

enum IOOps {
    // epollへ追加
    ADD(EpollFlags, RawFd, Waker),
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
        assert!(!wakers.contains_key(&fd));
        wakers.insert(fd, waker);
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
                            IOOps::ADD(flag, fd, waker) => self.add_event(flag, fd, waker, &mut t),
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

struct AsyncListener {
    listener: TcpListener,
    selector: Arc<IOSelector>,
}

impl AsyncListener {
    // TcpListenerの初期化処理をラップした関数
    fn listen(addr: &str, selector: Arc<IOSelector>) -> AsyncListener {
        let listener = TcpListener::bind(addr).unwrap();
        // ノンブロッキングに指定
        listener.set_nonblocking(true).unwrap();

        AsyncListener {
            listener: listener,
            selector: selector,
        }
    }

    // コネクションをアクセプトするためのFutureをリターン
    fn accept(&self) -> Accept {
        Accept { listener: self }
    }
}

impl Drop for AsyncListener {
    fn drop(&mut self) {
        self.selector.unregister(self.listener.as_raw_fd());
    }
}

struct Accept<'a> {
    listener: &'a AsyncListener,
}

impl<'a> Future for Accept<'a> {
    // 返り値の型
    type Output = (
        AsyncReader,          // 非同期読み込みストリーム
        BufWriter<TcpStream>, // 書き込みストリーム
        SocketAddr,
    ); // アドレス

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // アクセプトをノンブロッキングで実行
        match self.listener.listener.accept() {
            Ok((stream, addr)) => {
                // アクセプトした場合は、読み込みと書き込み用オブジェクトをリターン
                let stream0 = stream.try_clone().unwrap();
                Poll::Ready((
                    AsyncReader::new(stream0, self.listener.selector.clone()),
                    BufWriter::new(stream),
                    addr,
                ))
            }
            Err(err) => {
                // アクセプトすべきコネクションがない場合は、epollに登録
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    self.listener.selector.register(
                        EpollFlags::EPOLLIN,
                        self.listener.listener.as_raw_fd(),
                        cx.waker().clone(),
                    );
                    Poll::Pending
                } else {
                    panic!("accept: {}", err);
                }
            }
        }
    }
}

struct AsyncReader {
    fd: RawFd,
    reader: BufReader<TcpStream>,
    selector: Arc<IOSelector>,
}

impl AsyncReader {
    fn new(stream: TcpStream, selector: Arc<IOSelector>) -> AsyncReader {
        // ノンブロッキングに設定
        stream.set_nonblocking(true).unwrap();

        AsyncReader {
            fd: stream.as_raw_fd(),
            reader: BufReader::new(stream),
            selector: selector,
        }
    }

    fn read_line(&mut self) -> ReadLine {
        ReadLine { reader: self }
    }
}

impl Drop for AsyncReader {
    fn drop(&mut self) {
        self.selector.unregister(self.fd);
    }
}

struct ReadLine<'a> {
    reader: &'a mut AsyncReader,
}

impl<'a> Future for ReadLine<'a> {
    // 返り値の型
    type Output = Option<String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut line = String::new();
        // 非同期読み込み
        match self.reader.reader.read_line(&mut line) {
            Ok(0) => Poll::Ready(None),       // コネクションクローズ
            Ok(_) => Poll::Ready(Some(line)), // 1行読み込み成功
            Err(err) => {
                // 読み込みできない場合はepollに登録
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    self.reader.selector.register(
                        EpollFlags::EPOLLIN,
                        self.reader.fd,
                        cx.waker().clone(),
                    );
                    Poll::Pending
                } else {
                    Poll::Ready(None)
                }
            }
        }
    }
}

struct Task {
    // 実行するコルーチン
    future: Mutex<BoxFuture<'static, ()>>,
    // Executorへスケジューリングするためのチャネル
    sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // 自身をスケジューリング
        let self0 = arc_self.clone();
        arc_self.sender.send(self0).unwrap();
    }
}

struct Executor {
    // 実行キュー
    sender: SyncSender<Arc<Task>>,
    receiver: Receiver<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        // チャネルを生成
        let (sender, receiver) = sync_channel(1024);
        Executor {
            sender: sender.clone(),
            receiver,
        }
    }

    fn get_spawner(&self) -> Spawner {
        Spawner { sender: self.sender.clone() }
    }

    fn run(&self) {
        // チャネルからTaskを受信して順に実行
        while let Ok(task) = self.receiver.recv() {
            // コンテキストを作成
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let mut ctx = Context::from_waker(&waker);
            // pollを呼び出し実行
            let _ = future.as_mut().poll(&mut ctx);
        }
    }
}

struct Spawner {
    sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        // FutureをBox化
        let future = future.boxed();
        // Task生成
        let task = Arc::new(Task {    
            future: Mutex::new(future),
            sender: self.sender.clone(),
        });

        // 実行キューにエンキュー
        self.sender.send(task).unwrap();
    }
}

fn main() {
    let executor = Executor::new();
    let selector = IOSelector::new();
    let spawner = executor.get_spawner();

    let server = async move {
        let listener = AsyncListener::listen("127.0.0.1:10000", selector.clone());

        loop {
            // 非同期コネクションアクセプト
            let (mut reader, mut writer, addr) = listener.accept().await;
            println!("accept: {}", addr);

            // コネクション毎にタスクを作成
            spawner.spawn(async move {
                while let Some(buf) = reader.read_line().await {
                    print!("read: {}, {}", addr, buf);
                    writer.write(buf.as_bytes()).unwrap();
                    writer.flush().unwrap();
                }
                println!("close: {}", addr);
            });
        }
    };
    executor.get_spawner().spawn(server);
    executor.run();
}
