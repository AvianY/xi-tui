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

extern crate futures;
extern crate termion;
extern crate tokio_core;
extern crate xrl;

use tokio_core::reactor::Core;

mod tui;
mod window;
mod cache;
mod errors;
mod input;
mod view;
use tui::{Tui, TuiServiceBuilder};
use xrl::spawn;

use errors::*;
use log::LogLevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};

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

fn main() {
    let xi = clap_app!(
        xi =>
        (about: "The Xi Editor")
        (@arg core: -c --core +takes_value "Specify binary to use for the backend")
        (@arg logfile: -l --log-file +takes_value "Log file location")
        (@arg file: +required "File to edit"));

    let matches = xi.get_matches();
    let logfile = matches.value_of("logfile").unwrap_or("xi-tui.log");
    configure_logs(logfile);

    info!("START");
    let mut core = Core::new().unwrap();
    let (tui_builder, core_events_rx) = TuiServiceBuilder::new();
    let client = spawn("xi-core", tui_builder, &core.handle());
    let mut tui = Tui::new(core.handle(), client, core_events_rx);
    tui.open("Foo.txt");
    core.run(tui).unwrap();
}
