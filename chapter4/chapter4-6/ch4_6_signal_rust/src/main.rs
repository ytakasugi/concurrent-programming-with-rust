use signal_hook::iterator::Signals;
use std::{error::Error, process, thread, time::Duration};
use libc::SIGUSR1;

fn main() -> Result<(), Box<dyn Error>> {
    println!("pid: {}", process::id());
    
    let mut signal = Signals::new(&[SIGUSR1])?;
    thread::spawn(move || {
        // シグナル受信
        for s in signal.forever() {
            println!("received signal: {:?}", s);
        }
    });

    thread::sleep(Duration::from_secs(10));
    Ok(())
}