use num_cpus;
use std::io::{Result, Write};
use std::sync::atomic::{AtomicBool};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread::{self};
use threadpool::ThreadPool;
use crate::{VolumeInfo, FilterToken};
use crate::locate::{LocateSink, SelectionInsert, locate_volume};

enum Msg {
    Info(Vec<u8>),
    Error(Vec<u8>),
    Selection(Vec<u8>, Option<u64>),
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

struct SelectionProxy<'a> {
    send: &'a dyn Fn(Vec<u8>, Option<u64>),
}

impl<'a> SelectionProxy<'a> {
    fn new(send: &'a dyn Fn(Vec<u8>, Option<u64>)) -> SelectionProxy<'a> {
        SelectionProxy {
            send,
        }
    }
}

impl<'a> SelectionInsert for SelectionProxy<'a> {
    fn insert(&mut self, path: &[u8], size: Option<u64>) {
        let buf = path.to_vec();
        (self.send)(buf, size);
    }

    fn insert_owned(&mut self, path: Vec<u8>, size: Option<u64>) {
        (self.send)(path, size);
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
                let send_selection = |path: Vec<u8>, size: Option<u64>| {let _ = tx.send(Msg::Selection(path, size));};
                let mut stdout_proxy = Proxy::new(&send_info);
                let mut stderr_proxy = Proxy::new(&send_error);
                let mut selection_proxy = SelectionProxy::new(&send_selection);
                let mut inner_sink = LocateSink {
                    verbosity: sink.verbosity,
                    stdout: &mut stdout_proxy,
                    stderr: &mut stderr_proxy,
                    selection: &mut selection_proxy,
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
            Ok(Msg::Selection(path, size)) => {let _ = sink.selection.insert_owned(path, size);},
            Err(_) => {break;},
        };
    }

    handle.join().expect("join failed");
}
