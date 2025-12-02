mod config;
mod consts;
mod llm_request;
mod models;

use actix_web::web::Bytes;
use actix_web::{middleware::Logger, mime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::models::{model_list, request};
use std::time::Duration;

async fn models() -> impl actix_web::Responder {
    let mut model_list: Vec<model_list::Model> = vec![];

    for model_name in config::MODEL_MAPPING.keys() {
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
    request: actix_web::web::Json<request::ChatCompletionCreate>,
) -> impl actix_web::Responder {
    if let None = config::MODEL_MAPPING.get(&request.0.model) {
        println!("error: invalid model name");
        return actix_web::HttpResponse::BadRequest().finish();
    };

    let client = llm_request::LLMClient::new(reqwest::Client::new(), config::API_URL);
    println!("   ***   Request: {:?}\n   ***   ", request.0);

    if request.stream.unwrap_or(false) {
        let (sender, receiver) = mpsc::channel::<Result<Bytes, Box<dyn std::error::Error>>>(100);
        actix_web::rt::spawn(async move {
            llm_request::stream_chat_completion(&client, request.0, sender, Duration::new(300, 0))
                .await
        });

        return actix_web::HttpResponse::Ok()
            .content_type(mime::TEXT_EVENT_STREAM)
            .streaming(ReceiverStream::new(receiver));
    }

    match llm_request::create_chat_completion(&client, request.0, Duration::new(300, 0)).await {
        Ok(chat_completion) => actix_web::HttpResponse::Ok().json(chat_completion),
        Err(e) => {
            println!("error: {:?}", e);
            actix_web::HttpResponse::BadGateway().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Welcome!");

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let app_factory = || {
        let router = actix_web::web::scope("/v1")
            .route("/models", actix_web::web::get().to(models))
            .route(
                "/chat/completions",
                actix_web::web::post().to(chat_completion),
            );

        actix_web::App::new()
            .wrap(Logger::default())
            .service(router)
    };

    let server = actix_web::HttpServer::new(app_factory);

    server.bind(("0.0.0.0", 8080))?.run().await
}
