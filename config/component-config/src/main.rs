use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

mod server_config;
pub use server_config::*;

mod server_settings;
pub use server_settings::*;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let settings = Settings::load().unwrap_or_else(|err| {
        panic!("{}", format!("error={:#?}", err));
    });
    println!("settings={:#?}", settings);

    let cfg_route = warp::path("local").and(with_cfg(settings.clone())).and_then(cfg_handler);

    let mut port: u16 = 0;
    if let Some(ComponentConfig::EchoService(service)) = settings.component_config.get("EchoService") {
        port = service.server.port;
    }
    println!("Server started at localhost:{} and ENV: {}", port, settings.server_config.env);

    warp::serve(cfg_route).run(([0, 0, 0, 0], port)).await;
}

async fn cfg_handler(cfg: Settings) -> Result<impl Reply, Rejection> {
    let mut service = EchoService::default();
    if let Some(ComponentConfig::EchoService(svc)) = cfg.component_config.get("EchoService") {
        service = svc.clone();
    }
    Ok(format!("Running on port: {} with url: {}", service.server.port, service.server.url))
}

fn with_cfg(cfg: Settings) -> impl Filter<Extract = (Settings,), Error = Infallible> + Clone { warp::any().map(move || cfg.clone()) }

#[cfg(test)]
mod tests {}
