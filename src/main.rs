mod config;
mod consts;
mod errors;
mod llm_client;
mod llm_request;
mod models;
mod service;

use actix_web::web::{Bytes, Data, ThinData};
use actix_web::{middleware::Logger, mime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::errors::ReasonerError;
use crate::models::{model_list, request};
use std::sync::Arc;
use std::time::Duration;

async fn models(config: Data<config::Config>) -> impl actix_web::Responder {
    let mut model_list: Vec<model_list::Model> = vec![];

    for model_name in config.models.keys() {
        model_list.push(model_list::Model {
            id: model_name.to_string(),
            object: model_list::ObjectType::Model,
            created: 0,
            owned_by: model_list::Owner::AdaptiveReasoner,
        });
    }

    let model_list = model_list::ModelList { data: model_list };

    actix_web::HttpResponse::Ok().json(model_list)
}

async fn chat_completion(
    ThinData(client): ThinData<reqwest::Client>,
    config: Data<config::Config>,
    request: actix_web::web::Json<request::ChatCompletionCreate>,
) -> impl actix_web::Responder {
    let model_config = match config.models.get(&request.0.model).cloned() {
        Some(model_config) => model_config,
        None => {
            log::info!("error: model not found: {:?}", request.0.model);
            return actix_web::HttpResponse::BadRequest().finish();
        }
    };

    let client = llm_client::LLMClient::new(
        client,
        &model_config.api_url,
        &model_config.api_key,
        &model_config.extra,
    );
    log::debug!("request: {:?}", request.0);

    if request.stream.unwrap_or(false) {
        let (sender, receiver) = mpsc::channel::<Result<Bytes, ReasonerError>>(100);
        actix_web::rt::spawn(async move {
            if let Err(e) = llm_request::stream_chat_completion(&client, request.0, &model_config, sender).await {
                log::error!("stream_chat_completion error: {:?}", e);
            }
        });

        return actix_web::HttpResponse::Ok()
            .content_type(mime::TEXT_EVENT_STREAM)
            .streaming(ReceiverStream::new(receiver));
    }

    match llm_request::create_chat_completion(&client, request.0, &model_config).await {
        Ok(chat_completion) => actix_web::HttpResponse::Ok().json(chat_completion),
        Err(e) => {
            log::error!("create_chat_completion error: {:?}", e);
            actix_web::HttpResponse::BadGateway().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("Initializing Adaptive Reasoner service...");

    let model_config = config::load_config().expect("Failed to load config");
    let data = Data::from(Arc::new(model_config));

    let app_factory = move || {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::new(30, 0))
            .read_timeout(Duration::new(60, 0))
            .build()
            .unwrap();

        let router = actix_web::web::scope("/v1")
            .route("/models", actix_web::web::get().to(models))
            .route(
                "/chat/completions",
                actix_web::web::post().to(chat_completion),
            );

        actix_web::App::new()
            .wrap(Logger::default())
            .app_data(ThinData(client))
            .app_data(data.clone())
            .service(router)
    };

    let server = actix_web::HttpServer::new(app_factory);

    server.bind(("0.0.0.0", 8080))?.run().await
}
