extern crate toml;
extern crate rppal;
extern crate regex;
extern crate serde;
extern crate exitcode;
extern crate syslog;
#[macro_use]
extern crate log;

pub mod temp;
pub mod fan;

use serde::Deserialize;
use std::fmt::Debug;
use syslog::{Facility, Formatter3164, BasicLogger};
use log::LevelFilter;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct Config {
    fan : fan::Fan,
}

fn main() {
    let formatter = Formatter3164 {
        facility: Facility::LOG_DAEMON,
        hostname: None,
        process: "fan-service".into(),
        pid: std::process::id() as i32,
    };

    let logger = syslog::unix(formatter).expect("could not connect to syslog");
    match log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
            .map(|()| log::set_max_level(LevelFilter::Info))
    {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to set boxed loger: {:?}", e);
            std::process::exit(exitcode::IOERR);
        },   
    }

    let locations = vec![
        String::from("/etc/fan-service/settings.toml")
    ];
    
    let config : Config = match search_config(&locations[..]) {
        Ok(string) => {
            match toml::from_str(string.as_str()) {
                Ok(toml) => toml,
                Err(e) => {
                    error!("{:?}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }
        },
        Err(e) => {
            error!("{:?}", e);
            std::process::exit(exitcode::IOERR);
        }
    };

    match config.fan.init() {
        Ok(mut fan) => {
            let mut prev_temp = 0;
            loop {
                match temp::get_temperature() {
                    Ok(temp) => {
                        let temp = temp.round() as i16;
                        if (temp - prev_temp).abs() >= 1 {
                            match fan.update(temp as u8) {
                                Ok(_) => {
                                    prev_temp = temp;
                                },
                                Err(e) => {
                                    error!("Unable to set pwm: {:?}", e);
                                    std::process::exit(exitcode::IOERR);
                                }
                            }
                            
                        }
                    }
                    Err(e) => {
                        error!("Unable to get temperature: {:?}", e);
                        std::process::exit(exitcode::IOERR);
                    }
                }
                
                std::thread::sleep(std::time::Duration::from_millis(config.fan.update_rate().into()))
            }
        },
        Err(e) => {
            error!("Failed to initialize fan: {:?}", e);
            std::process::exit(exitcode::IOERR);
        }
    }
}

fn search_config(locations : &[String]) -> std::io::Result<String> {
    for location in locations {
        if Path::new(location).exists() {
            return std::fs::read_to_string(location);
        }
    }
    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found in any location"));
}
