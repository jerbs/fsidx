use num_cpus;
use std::io::{Result, Write};
use std::sync::atomic::{AtomicBool};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread::{self};
use threadpool::ThreadPool;
use crate::{VolumeInfo, FilterToken};
use crate::locate::{LocateSink, locate_volume};

enum Msg {
    Info(Vec<u8>),
    Error(Vec<u8>)
}

struct Proxy<'a> {
    send: &'a dyn Fn(&[u8]),
    // sender: Sender<Msg>,
    buffer: Vec<u8>,
}

impl<'a> Proxy<'a> {
    fn new(send: &'a dyn Fn(&[u8])) -> Proxy<'a> {
        Proxy {
            send,
            buffer: Vec::new(),
        }
    }
}

impl<'a> Write for Proxy<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.buffer.extend(buf.iter());
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        let buf = core::mem::take(&mut self.buffer);
        (self.send)(&buf);
        Ok(())
    }
}

pub fn locate_mt(volume_info: Vec<VolumeInfo>, filter: Vec<FilterToken>, sink: LocateSink, interrupt: Option<Arc<AtomicBool>>) {
    let num_cpu_cores = num_cpus::get();
    // let _ = writeln!(sink.stdout, "Num CPU Cores: {}", num_cpu_cores);
    let(tx, rx) = channel();

    let handle = thread::spawn(move|| {
        let pool = ThreadPool::new(num_cpu_cores);
        for vi in &volume_info {
            let tx = tx.clone();
            let vi = vi.clone();
            let filter = filter.clone();
            let interrupt = interrupt.clone();
            pool.execute(move|| {
                let ty = tx.clone();
                let send_info  = |buf: &[u8]| {let _ = ty.send(Msg::Info(buf.to_vec()));};
                let send_error = |buf: &[u8]| {let _ = tx.send(Msg::Error(buf.to_vec()));};
                let mut stdout_proxy = Proxy::new(&send_info);
                let mut stderr_proxy = Proxy::new(&send_error);
                let mut inner_sink = LocateSink {
                    stdout: &mut stdout_proxy,
                    stderr: &mut stderr_proxy,
                };
                let _ = locate_volume(&vi, &filter, &mut inner_sink, interrupt);
                let _ = stdout_proxy.flush();
                let _ = stderr_proxy.flush();
            });
        }
    });

    loop {
        let recv = rx.recv();
        match recv {
            Ok(Msg::Info(text)) => {let _ = sink.stdout.write_all(&text);},
            Ok(Msg::Error(text)) => {let _ = sink.stderr.write_all(&text);},
            Err(_) => {break;},
        };
    }

    handle.join().expect("join failed");
}
