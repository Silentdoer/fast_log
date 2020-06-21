use std::cell::{Cell, RefCell};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::RecvError;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Local};
use log::{error, info, warn};
use log::{Level, LevelFilter, Metadata, Record};
use tokio::prelude::*;
use crate::error::LogError;
use crate::time_util;

/// debug mode,true:print to console, false ,only write to file.
pub static DEBUG_MODE: AtomicBool = AtomicBool::new(true);

lazy_static! {
   static ref LOG:RwLock<Option<SimpleLogger>>=RwLock::new(Option::None);
}


pub struct LoggerRecv {
    //std recv
    pub std_recv: Option<std::sync::mpsc::Receiver<String>>,
}


pub struct SimpleLogger {
    pub runtime_type: RuntimeType,
    //std sender
    pub std_sender: Option<std::sync::mpsc::SyncSender<String>>,
}

///runtime Type
#[derive(Clone, Debug)]
pub enum RuntimeType {
    Std
}


impl SimpleLogger {
    pub fn new(runtime_type: RuntimeType) -> (Self, LoggerRecv) {
        return match runtime_type {
            _ => {
                let (s, r) = std::sync::mpsc::sync_channel(1000);
                (Self {
                    runtime_type,
                    std_sender: Some(s),
                }, LoggerRecv { std_recv: Some(r) })
            }
        };
    }
    pub fn send(&self, arg: String) {
        match self.runtime_type {
            _ => {
                self.std_sender.as_ref().unwrap().send(arg);
            }
        }
    }
}

fn set_log(runtime_type: RuntimeType) -> LoggerRecv {
    let mut w = LOG.write().unwrap();
    let (log, recv) = SimpleLogger::new(runtime_type);
    *w = Some(log);
    return recv;
}


pub struct Logger {}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut module = "";
            if record.module_path().is_some() {
                module = record.module_path().unwrap();
            }
            let local: DateTime<Local> = Local::now();
            let data = format!("{:?} {} {} - {}  {}", local, record.level(), module, record.args(), format_line(record));

            let debug = DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed);
            if debug {
                print!("{}\n", data.as_str());
            }
            //send
            LOG.read().unwrap().as_ref().unwrap().send(data);
        }
    }
    fn flush(&self) {}
}

fn format_line(record: &Record<'_>) -> String {
    match (record.file(), record.line()) {
        (Some(file), Some(line)) => format!("({}:{})", file, line),
        _ => String::new(),
    }
}

static LOGGER: Logger = Logger {};


/// initializes the log file path
/// log_file_path for example "test.log"
/// 初始化日志文件路径
/// log_file_path 文件路径 例如 "test.log"
pub fn init_log(log_file_path: &str, runtime_type: &RuntimeType) -> Result<(), Box<dyn std::error::Error + Send>> {
    let recv = set_log(runtime_type.clone());
    let log_path = log_file_path.to_owned();
    let mut file = OpenOptions::new().create(true).append(true).open(log_path.as_str());
    if file.is_err() {
        file = File::create(Path::new(log_path.as_str()));
    }
    if file.is_err() {
        println!("[log] the log path:{} is not true!", log_path.as_str());
        let e = LogError::from(format!("[log] the log path:{} is not true!", log_path.as_str()).as_str());
        return Err(Box::new(e));
    }
    let mut file = file.unwrap();
    std::thread::spawn(move || {
        loop {
            //recv
            let data = recv.std_recv.as_ref().unwrap().recv();
            if data.is_ok() {
                let s: String = data.unwrap() + "\n";
                file.write(s.as_bytes());
            }
        }
    });
    let r = log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info));
    if r.is_err() {
        return Err(Box::new(r.err().unwrap()));
    } else {
        return Ok(());
    }
}


//cargo test --release --color=always --package fast_log --lib log::bench_log --all-features -- --nocapture --exact
#[test]
pub fn bench_log() {
    init_log("requests.log", &RuntimeType::Std);
    // DEBUG_MODE.store(false,std::sync::atomic::Ordering::Relaxed);
    let total = 10000;
    let now = SystemTime::now();
    for i in 0..total {
        //sleep(Duration::from_secs(1));
        info!("Commencing yak shaving");
    }
    time_util::count_time_tps(total, now);
}
