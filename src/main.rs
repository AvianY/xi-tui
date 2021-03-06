#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", deny(clippy))]

#[macro_use]
extern crate clap;

#[macro_use]
extern crate error_chain;

extern crate log4rs;
#[macro_use]
extern crate log;

extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

extern crate termion;

mod core;
mod cursor;
mod window;
mod cache;
mod errors;
mod input;
mod line;
mod operation;
mod screen;
mod style;
mod update;
mod view;

use error_chain::ChainedError;

use core::Core;
use errors::*;
use input::Input;
use screen::Screen;
use log::LogLevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();

        writeln!(stderr, "error: {}", e).unwrap();

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).unwrap();
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).unwrap();
        }
        ::std::process::exit(1);
    }
}

fn configure_logs(logfile: &str) {
    let file_appender = FileAppender::builder().build(logfile).unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .logger(Logger::builder().build("xi_tui::core", LogLevelFilter::Debug))
        .logger(Logger::builder().build("xi_tui::main", LogLevelFilter::Debug))
        .build(Root::builder().appender("file").build(LogLevelFilter::Info))
        .unwrap();
    let _ = log4rs::init_config(config).unwrap();
}
fn run() -> Result<()> {
    let xi = clap_app!(
        xi =>
        (about: "The Xi Editor")
        (@arg core: -c --core +takes_value "Specify binary to use for the backend")
        (@arg logfile: -l --log-file +takes_value "Log file location")
        (@arg file: +required "File to edit"));

    let matches = xi.get_matches();
    let core_exe = matches.value_of("core").unwrap_or("xi-core");
    let logfile = matches.value_of("logfile").unwrap_or("xi-tui.log");
    let file = matches.value_of("file").unwrap();

    configure_logs(logfile);
    let mut core = Core::new(core_exe);
    let mut screen = Screen::new()?;
    let mut input = Input::new();
    input.run();
    screen.init()?;
    core.open(file)?;
    loop {
        match screen.resize() {
            Ok(Some(new_size)) => {
                info!("screen height changed. Notifying the core");
                core.resize(new_size.1)?;
                screen.schedule_update();
            }
            Err(e) => {
                error!("failed to get new screen size");
                log_error(&e);
            }
            _ => {}
        }

        if let Ok(event) = input.try_recv() {
            if let Err(e) = input::handle(&event, &mut core) {
                log_error(&e);
            }
        } else if let Err(e) = screen.update(&mut core) {
            log_error(&e);
        }
    }
}

fn log_error<E: ChainedError>(e: &E) {
    error!("error: {}", e);
    for e in e.iter().skip(1) {
        error!("caused by: {}", e);
    }
}
