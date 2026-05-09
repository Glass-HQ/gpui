use gpui::{App, Application};
use gpui_examples::ios::{available_demo_names, run_demo_named};
use log::LevelFilter;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static STARTED: AtomicBool = AtomicBool::new(false);

struct TcpSinkState {
    stream: Option<TcpStream>,
    last_reconnect_attempt_ms: u64,
}

static TCP_SINK: Mutex<Option<TcpSinkState>> = Mutex::new(None);
static RELAY_ADDR: Mutex<Option<SocketAddr>> = Mutex::new(None);

const TCP_RECONNECT_COOLDOWN_MS: u64 = 5_000;

struct IosLogger {
    subsystem: String,
}

impl IosLogger {
    fn new(subsystem: &str) -> Self {
        Self {
            subsystem: subsystem.to_string(),
        }
    }

    fn level_color(level: log::Level) -> &'static str {
        match level {
            log::Level::Error => "\x1b[31m",
            log::Level::Warn => "\x1b[33m",
            log::Level::Info => "\x1b[32m",
            log::Level::Debug => "\x1b[36m",
            log::Level::Trace => "\x1b[90m",
        }
    }

    fn level_tag(level: log::Level) -> &'static str {
        match level {
            log::Level::Error => "ERROR",
            log::Level::Warn => "WARN ",
            log::Level::Info => "INFO ",
            log::Level::Debug => "DEBUG",
            log::Level::Trace => "TRACE",
        }
    }

    fn timestamp() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let total_secs = now.as_secs();
        let millis = now.subsec_millis();
        let hours = (total_secs / 3600) % 24;
        let minutes = (total_secs / 60) % 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl log::Log for IosLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let message = format!("{}", record.args());
        let ts = Self::timestamp();

        let os_log = oslog::OsLog::new(&self.subsystem, record.target());
        os_log.with_level(record.level().into(), &message);

        let color = Self::level_color(record.level());
        let reset = "\x1b[0m";
        let tag = Self::level_tag(record.level());
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(
            stderr,
            "{ts} {color}{tag}{reset} [{}] {}",
            record.target(),
            message,
        );
        let _ = stderr.flush();

        if let Ok(mut guard) = TCP_SINK.lock() {
            if let Some(ref mut sink) = *guard {
                let line = format!(
                    "{ts} {color}{tag}{reset} [{}] {}\n",
                    record.target(),
                    message,
                );
                if let Some(ref mut stream) = sink.stream {
                    if stream.write_all(line.as_bytes()).is_err() {
                        sink.stream = None;
                        try_reconnect_tcp(sink);
                    }
                } else {
                    try_reconnect_tcp(sink);
                    if let Some(ref mut stream) = sink.stream {
                        let _ = stream.write_all(line.as_bytes());
                    }
                }
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
        if let Ok(mut guard) = TCP_SINK.lock() {
            if let Some(ref mut sink) = *guard {
                if let Some(ref mut stream) = sink.stream {
                    let _ = stream.flush();
                }
            }
        }
    }
}

fn try_reconnect_tcp(sink: &mut TcpSinkState) {
    let now = IosLogger::now_ms();
    if now.saturating_sub(sink.last_reconnect_attempt_ms) < TCP_RECONNECT_COOLDOWN_MS {
        return;
    }
    sink.last_reconnect_attempt_ms = now;

    let addr = match RELAY_ADDR.lock() {
        Ok(guard) => match *guard {
            Some(a) => a,
            None => return,
        },
        Err(_) => return,
    };

    if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
        let _ = stream.set_nodelay(true);
        sink.stream = Some(stream);
    }
}

fn try_connect_log_relay() {
    let addr = match option_env!("GPUI_LOG_RELAY") {
        Some(a) if !a.is_empty() => a,
        _ => return,
    };

    let sock_addr = match addr.parse::<SocketAddr>() {
        Ok(a) => a,
        Err(_) => return,
    };

    if let Ok(mut guard) = RELAY_ADDR.lock() {
        *guard = Some(sock_addr);
    }

    match TcpStream::connect_timeout(&sock_addr, Duration::from_secs(2)) {
        Ok(stream) => {
            let _ = stream.set_nodelay(true);
            *TCP_SINK.lock().unwrap() = Some(TcpSinkState {
                stream: Some(stream),
                last_reconnect_attempt_ms: 0,
            });
        }
        Err(_) => {
            *TCP_SINK.lock().unwrap() = Some(TcpSinkState {
                stream: None,
                last_reconnect_attempt_ms: 0,
            });
        }
    }
}

fn init_logging(subsystem: &str) {
    try_connect_log_relay();
    let logger = IosLogger::new(subsystem);
    log::set_boxed_logger(Box::new(logger)).expect("failed to set logger");

    let level = match option_env!("GPUI_LOG_LEVEL") {
        Some("trace") | Some("TRACE") => LevelFilter::Trace,
        Some("debug") | Some("DEBUG") => LevelFilter::Debug,
        Some("info") | Some("INFO") => LevelFilter::Info,
        Some("warn") | Some("WARN") => LevelFilter::Warn,
        Some("error") | Some("ERROR") => LevelFilter::Error,
        _ => LevelFilter::Debug,
    };
    log::set_max_level(level);
}

fn initialize_demo(subsystem: &str) {
    init_logging(subsystem);
    std::panic::set_hook(Box::new(|info| {
        log::error!("[GPUI-iOS] PANIC: {}", info);
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/Documents/gpui_panic.log", home);
        let _ = std::fs::write(&path, format!("{}", info));
    }));
    log::info!("[GPUI-iOS] launching app ({subsystem})");
}

fn launch_app(setup: Box<dyn FnOnce(&mut App)>) {
    let app = Application::with_platform(Rc::new(crate::IosPlatform::new(false)));
    let keepalive = app.clone();
    let _ = Box::leak(Box::new(keepalive));
    app.run(move |cx| setup(cx));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_run_demo(name: *const std::ffi::c_char) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("hello_world");

    initialize_demo("dev.glasshq.GPUIiOS");
    if run_demo_named(name, &launch_app).is_err() {
        log::error!(
            "Unknown demo: '{}'. Available: {}",
            name,
            available_demo_names().collect::<Vec<_>>().join(", ")
        );
        let _ = run_demo_named("hello_world", &launch_app);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_ios_list_demos() -> *mut std::ffi::c_char {
    let list = available_demo_names().collect::<Vec<_>>().join("\n");
    std::ffi::CString::new(list)
        .expect("demo names contain no NUL bytes")
        .into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_free_string(s: *mut std::ffi::c_char) {
    if !s.is_null() {
        unsafe { drop(std::ffi::CString::from_raw(s)) };
    }
}
