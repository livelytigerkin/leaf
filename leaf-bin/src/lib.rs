use std::ffi::CStr;
use std::os::raw::c_char;
use std::process::exit;

use clap::{App, Arg};
use log::*;

use leaf::config;
use leaf::config::internal;
use leaf::proxy::tun::inbound::{get_read_traffic, get_write_traffic};

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const COMMIT_HASH: Option<&'static str> = option_env!("CFG_COMMIT_HASH");
const COMMIT_DATE: Option<&'static str> = option_env!("CFG_COMMIT_DATE");

fn get_version_string() -> String {
    match (VERSION, COMMIT_HASH, COMMIT_DATE) {
        (Some(ver), None, None) => ver.to_string(),
        (Some(ver), Some(hash), Some(date)) => format!("{} ({} - {})", ver, hash, date),
        _ => "unknown".to_string(),
    }
}

#[cfg(debug_assertions)]
fn default_thread_stack_size() -> usize {
    2 * 1024 * 1024
}

#[cfg(not(debug_assertions))]
fn default_thread_stack_size() -> usize {
    128 * 1024
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub extern "C" fn run_kumquat(conf_string: *const c_char) {

    let mut config: internal::Config;
    if let Ok(valid_config) = unsafe { CStr::from_ptr(conf_string).to_str() }
        .map_err(Into::into)
        .and_then(config::from_conf_string) {
        config = valid_config
    } else {
        error!("invalid config path or config file");
        return
    }

    let stack_size = default_thread_stack_size();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(stack_size)
        .enable_all()
        .build()
        .unwrap();

    let loglevel = if let Some(log) = config.log.as_ref() {
        match log.level {
            config::Log_Level::TRACE => log::LevelFilter::Trace,
            config::Log_Level::DEBUG => log::LevelFilter::Debug,
            config::Log_Level::INFO => log::LevelFilter::Info,
            config::Log_Level::WARN => log::LevelFilter::Warn,
            config::Log_Level::ERROR => log::LevelFilter::Error,
        }
    } else {
        log::LevelFilter::Info
    };
    let mut logger = leaf::common::log::setup_logger(loglevel);
    let console_output = fern::Output::stdout("\n");
    logger = logger.chain(console_output);
    if let Some(log) = config.log.as_ref() {
        match log.output {
            config::Log_Output::CONSOLE => {
                // console output already applied
            }
            config::Log_Output::FILE => {
                let f = fern::log_file(&log.output_file).expect("open log file failed");
                let file_output = fern::Output::file(f, "\n");
                logger = logger.chain(file_output);
            }
        }
    }
    leaf::common::log::apply_logger(logger);

    let runners = match leaf::util::create_runners(config) {
        Ok(v) => v,
        Err(e) => {
            error!("create runners fialed: {}", e);
            return;
        }
    };

    rt.block_on(async move {
        tokio::select! {
            _ = futures::future::join_all(runners) => (),
            // _ = tokio::signal::ctrl_c() => {
            //     warn!("ctrl-c received, exit");
            // },
        }
    });
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub extern "C" fn get_read_traffics() -> u64 {
    return get_read_traffic()
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub extern "C" fn get_write_traffics() -> u64 {
    return get_write_traffic()
}
