use std::{ffi::CStr, os::raw::c_char};

use bytes::BytesMut;
use log::*;

use leaf::config;
use leaf::proxy::tun::inbound::{get_read_traffic, get_write_traffic};

#[cfg(any(target_os = "ios", target_os = "android"))]
pub mod bindings;

mod logger;

use logger::ConsoleWriter;

// This function is only available on iOS 13.0+
// use ios::os_proc_available_memory;

fn run(rt: &tokio::runtime::Runtime, config: leaf::config::Config) {
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
    let console_output = fern::Output::writer(Box::new(ConsoleWriter(BytesMut::new())), "\n");
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

    // let monit_mem = Box::pin(async {
    //     loop {
    //         let n = unsafe { os_proc_available_memory() };
    //         debug!("{} bytes memory available", n);
    //         tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    //     }
    // });

    rt.block_on(async move {
        tokio::select! {
            _ = futures::future::join_all(runners) => (),
            // _ = monit_mem  => (),
        }
    });
}

// #[cfg(target_os = "ios")]
// #[no_mangle]
// pub extern "C" fn run_leaf(config_path: *const c_char) {
//     let rt = tokio::runtime::Builder::new_current_thread()
//         .enable_all()
//         .build()
//         .unwrap();
//     if let Ok(config) = unsafe { CStr::from_ptr(config_path).to_str() }
//         .map_err(Into::into)
//         .and_then(leaf::config::from_file)
//     {
//         run(&rt, config);
//     } else {
//         error!("invalid config path or config file");
//         return;
//     }
// }

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn run_leaf(config_path: *const c_char, protect_path: *const c_char) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if let Ok(protect_path) = unsafe { CStr::from_ptr(protect_path).to_str() } {
        rt.block_on(leaf::proxy::set_socket_protect_path(
            protect_path.to_string(),
        ));
    }
    if let Ok(config) = unsafe { CStr::from_ptr(config_path).to_str() }
        .map_err(Into::into)
        .and_then(leaf::config::from_file)
    {
        run(&rt, config);
    } else {
        error!("invalid config path or config file");
        return;
    }
}

#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn run_kumquat(conf_string: *const c_char) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if let Ok(config) = unsafe { CStr::from_ptr(conf_string).to_str() }
	.map_err(Into::into)
        .and_then(leaf::config::from_conf_string)
    {
        run(&rt, config);
    } else {
        error!("invalid config path or config file");
        return;
    }
}

#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn get_read_traffics() -> u64 {
    return get_read_traffic()
}

#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn get_write_traffics() -> u64 {
    return get_write_traffic()
}
