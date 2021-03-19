use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

mod server_config;
pub use server_config::*;

mod server_settings;
pub use server_settings::*;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let (meta, config) = ServerSettings::load().unwrap_or_else(|err| {
        panic!("{}", format!("error={:#?}", err));
    });
    println!("meta={:#?}, settings={:#?}", meta, config);

    let cfg_route = warp::path("local").and(with_cfg(config.clone())).and_then(cfg_handler);

    println!("Server started at localhost:{} and ENV: {}", config.server.port, config.env);

    warp::serve(cfg_route).run(([0, 0, 0, 0], config.server.port)).await;
}

async fn cfg_handler(cfg: ServerSettings) -> Result<impl Reply, Rejection> {
    Ok(format!("Running on port: {} with url: {}", cfg.server.port, cfg.server.url))
}

fn with_cfg(cfg: ServerSettings) -> impl Filter<Extract = (ServerSettings,), Error = Infallible> + Clone {
    warp::any().map(move || cfg.clone())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
