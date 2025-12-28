mod app;
mod config;
mod consts;
mod errors;
mod handlers;
mod llm_client;
mod llm_request;
mod models;
mod service;

use std::sync::Arc;
use std::time::Duration;

use crate::app::create_app;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("Initializing Adaptive Reasoner service...");

    let model_config = config::load_config().expect("Failed to load config");

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::new(30, 0))
        .read_timeout(Duration::new(60, 0))
        .build()
        .unwrap();

    let reasoning_service = Arc::new(service::ReasoningService::new(http_client));
    let config = Arc::new(model_config);

    let app_factory = move || create_app(reasoning_service.clone(), config.clone());

    let server = actix_web::HttpServer::new(app_factory);

    server.bind(("0.0.0.0", 8080))?.run().await
}
