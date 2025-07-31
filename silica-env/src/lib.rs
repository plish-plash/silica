use std::{
    fs::{File, OpenOptions},
    io::Write,
    panic::PanicHookInfo,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug)]
pub struct AppInfo {
    pub package_name: &'static str,
    pub package_version: &'static str,
}

#[macro_export]
macro_rules! app_info {
    () => {
        $crate::AppInfo {
            package_name: env!("CARGO_PKG_NAME"),
            package_version: env!("CARGO_PKG_VERSION"),
        }
    };
}

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync>;

static APP_INFO: OnceLock<AppInfo> = OnceLock::new();
static DEFAULT_PANIC_HOOK: OnceLock<PanicHook> = OnceLock::new();
static HAS_PANICKED: AtomicBool = AtomicBool::new(false);

fn panic_hook(panic_info: &PanicHookInfo) {
    const CRASH_LOG_FILE: &str = "CRASH.txt";
    if let Some(default_hook) = DEFAULT_PANIC_HOOK.get() {
        default_hook(panic_info);
    }
    if let Some(app_info) = APP_INFO.get() {
        let result = (|| {
            if HAS_PANICKED.swap(true, Ordering::Relaxed) {
                let mut output = OpenOptions::new().append(true).open(CRASH_LOG_FILE)?;
                writeln!(output)?;
                writeln!(output, "{panic_info}")
            } else {
                let mut output = File::create(CRASH_LOG_FILE)?;
                writeln!(
                    output,
                    "{} v{}",
                    app_info.package_name, app_info.package_version
                )?;
                writeln!(
                    output,
                    "Running on {} {}",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                )?;
                writeln!(output)?;
                writeln!(output, "{panic_info}")
            }
        })();
        match result {
            Ok(()) => eprintln!("panic message written to {CRASH_LOG_FILE}"),
            Err(error) => eprintln!("failed to write {CRASH_LOG_FILE}: {error}"),
        }
    }
}

/// Initializes env_logger with appropriate filter levels and prints some info.
pub fn setup_logger(app_info: &AppInfo) {
    env_logger::builder()
        .filter_level(if cfg!(debug_assertions) {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        })
        .filter_module("calloop", log::LevelFilter::Info)
        .filter_module("wgpu_core", log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Info)
        .filter_module("cosmic_text", log::LevelFilter::Info)
        .parse_default_env()
        .init();

    log::info!("{} v{}", app_info.package_name, app_info.package_version);
    log::info!(
        "Running on {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// Sets the current directory to the executable's location.
pub fn setup_cwd() {
    let mut exe_dir = std::env::current_exe().expect("could not get path of current executable");
    exe_dir.pop();
    std::env::set_current_dir(&exe_dir).expect("failed to set current directory");
}

/// Sets a panic hook that writes panic messages to CRASH.txt.
pub fn setup_panic_hook(app_info: AppInfo) {
    let _ = APP_INFO.set(app_info);
    let _ = DEFAULT_PANIC_HOOK.set(std::panic::take_hook());
    std::panic::set_hook(Box::new(panic_hook));
}

#[cfg(debug_assertions)]
pub fn setup_env(app_info: AppInfo) {
    setup_logger(&app_info);
    if app_info.package_name.starts_with("silica-") {
        // set correct cwd for examples
        std::env::set_current_dir(format!("{}/examples", app_info.package_name))
            .expect("failed to set current directory");
    }
    setup_panic_hook(app_info);
}

#[cfg(not(debug_assertions))]
pub fn setup_env(app_info: AppInfo) {
    setup_cwd_release();
    setup_panic_hook(app_info);
}

pub fn get_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| {
        log::warn!("failed to get system locale, falling back to en-US");
        "en-US".to_string()
    })
}
