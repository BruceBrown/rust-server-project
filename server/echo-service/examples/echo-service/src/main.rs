use components::*;
use config::ConfigError;
use config_service::Settings;
use echo_service::EchoService;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};
use std::{error::Error, str::FromStr};

fn main() { main_().ok(); }

fn main_() -> Result<(), Box<dyn Error>> {
    let level_filter = <log::LevelFilter as FromStr>::from_str("debug").unwrap();
    CombinedLogger::init(vec![TermLogger::new(level_filter, Config::default(), TerminalMode::Mixed)]).unwrap();

    let mut services = load_services()?;
    for s in services.iter_mut() {
        if let Err(err) = s.start() {
            println!("Service {} failed to start, error={:#?}", s.get_name(), err);
            s.stop().ok();
        }
    }

    // Get the services running.
    for s in services.iter_mut() {
        if let Err(err) = s.run() {
            println!("Service {} failed to run, error={:#?}", s.get_name(), err);
            s.stop().ok();
        }
    }

    // Sit here for a while while clients come and go.
    std::thread::sleep(std::time::Duration::from_secs(10));
    // Drain the services.
    for s in services.iter_mut() {
        if let Err(err) = s.drain() {
            println!("Service {} failed to drain, error={:#?}", s.get_name(), err);
            s.stop().ok();
        }
    }
    // Wait for services to finish draining, but not too long.
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(5 * 60) {
        let drained = services.iter().all(|service| service.is_drained());
        if drained {
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    // Stop any that haven't already stopped.
    for s in services.iter_mut() {
        if let Err(err) = s.stop() {
            println!("Service {} failed to stop, error={:#?}", s.get_name(), err);
        }
    }
    Ok(())
}

fn load_services() -> Result<Vec<Box<dyn ServerService>>, ConfigError> {
    let mut services: Vec<Box<dyn ServerService>> = Vec::new();
    let settings = Settings::load()?;
    for f in &settings.server_config.features {
        match f.as_str() {
            "EchoService" => {
                let cfg = settings.service_config.get(f).ok_or_else(|| ConfigError::NotFound(f.clone()))?;
                let svc = EchoService::create(cfg, &settings)
                    .ok_or_else(|| ConfigError::Message("Incorrect settings for EchoService".to_string()))?;
                services.push(svc);
            },
            &_ => (),
        }
    }
    Ok(services)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_services() {
        match load_services() {
            Ok(services) => assert_ne!(0, services.len()),
            Err(err) => println!("Err={:#?}", err),
        }
    }
}
