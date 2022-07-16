use crate::thread::get_thread_name;
use chrono::Local;
use lazy_static::lazy_static;
use std::fmt::Arguments;
use std::io::Write;
use std::sync;
use std::{fmt, thread};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub enum Severity {
    Verbose,
    Info,
    Warn,
    Error,
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Severity::Verbose => write!(f, "verbose"),
            Severity::Info => write!(f, "info"),
            Severity::Warn => write!(f, "warn"),
            Severity::Error => write!(f, "error"),
            Severity::Fatal => write!(f, "fatal"),
        }
    }
}

pub struct Message {
    severity: Severity,
    crate_name: String,
    message: String,
    time: chrono::DateTime<Local>,
    thread: thread::ThreadId,
}

/// Implement a "sink". This receives log messages from the global logger and process them.
/// E.g: print to a file
pub trait Sink: Send {
    fn log(&self, message: &Message);
}

lazy_static! {
    static ref SINKS: sync::Mutex<Vec<Box<dyn Sink>>> = sync::Mutex::new(Vec::new());
}

#[doc(hidden)]
pub fn internal_log(severity: Severity, crate_name: &str, args: Arguments) {
    let str = args.to_string();
    let message = Message {
        severity,
        crate_name: crate_name.to_string(),
        message: str,
        time: Local::now(),
        thread: thread::current().id(),
    };

    for sink in SINKS.lock().unwrap().iter() {
        sink.log(&message);
    }

    if matches!(message.severity, Severity::Fatal) {
        panic!("{}", message.message);
    }
}

/** Sink API */

/**
 * Register a new sink
 */
pub fn register_sink(sink: Box<dyn Sink>) {
    SINKS.lock().unwrap().push(sink);
}

/** Default logging macros */
#[macro_export]
macro_rules! prism_verbose {
    ($($arg:tt)*) => ({
        $crate::logger::internal_log($crate::logger::Severity::Verbose, env!("CARGO_PKG_NAME"), format_args!($($arg)*));
    })
}

#[macro_export]
macro_rules! ze_info {
    ($($arg:tt)*) => ({
        $crate::logger::internal_log($crate::logger::Severity::Info, env!("CARGO_PKG_NAME"), format_args!($($arg)*));
    })
}

#[macro_export]
macro_rules! ze_warn {
    ($($arg:tt)*) => ({
        $crate::logger::internal_log($crate::logger::Severity::Warn, env!("CARGO_PKG_NAME"), format_args!($($arg)*));
    })
}

#[macro_export]
macro_rules! ze_error {
    ($($arg:tt)*) => ({
        $crate::logger::internal_log($crate::logger::Severity::Error, env!("CARGO_PKG_NAME"), format_args!($($arg)*));
    })
}

#[macro_export]
macro_rules! ze_fatal {
    ($($arg:tt)*) => ({
        $crate::logger::internal_log($crate::logger::Severity::Fatal, env!("CARGO_PKG_NAME"), format_args!($($arg)*));
        unreachable!();
    })
}

/** Default sinks */
#[derive(Default)]
pub struct StdoutSink;

impl Sink for StdoutSink {
    fn log(&self, message: &Message) {
        let mut stdout = StandardStream::stdout(ColorChoice::Auto);
        let thread_name = {
            match get_thread_name(message.thread) {
                None => "Unknown Thread".to_string(),
                Some(str) => str.as_ref().clone(),
            }
        };

        stdout
            .set_color(
                ColorSpec::new().set_fg(Option::from(match message.severity {
                    Severity::Verbose => Color::Cyan,
                    Severity::Info => Color::White,
                    Severity::Warn => Color::Yellow,
                    Severity::Error => Color::Red,
                    Severity::Fatal => Color::Rgb(255, 15, 15),
                })),
            )
            .unwrap();

        writeln!(
            &mut stdout,
            "[{}] [{}/{}] ({}) {}",
            message.time.format("%H:%M:%S"),
            message.severity,
            thread_name,
            message.crate_name,
            message.message
        )
        .unwrap();
        stdout.flush().unwrap();
    }
}
