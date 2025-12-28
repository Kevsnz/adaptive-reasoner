use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Data};
use actix_web::mime;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::config;
use crate::errors::ReasonerError;
use crate::models::{model_list, request};
use crate::service::ReasoningService;

pub async fn models(config: Data<config::Config>) -> impl actix_web::Responder {
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

pub async fn chat_completion(
    service: Data<ReasoningService>,
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

    log::debug!("request: {:?}", request.0);

    if request.stream.unwrap_or(false) {
        let (sender, receiver) = mpsc::channel::<Result<Bytes, ReasonerError>>(100);
        actix_web::rt::spawn(async move {
            if let Err(e) = service
                .stream_completion(request.0, &model_config, sender)
                .await
            {
                log::error!("stream_chat_completion error: {:?}", e);
            }
        });

        return actix_web::HttpResponse::Ok()
            .content_type(mime::TEXT_EVENT_STREAM)
            .streaming(ReceiverStream::new(receiver));
    }

    match service.create_completion(request.0, &model_config).await {
        Ok(chat_completion) => actix_web::HttpResponse::Ok().json(chat_completion),
        Err(e) => {
            log::error!("create_chat_completion error: {:?}", e);
            let status = match e {
                ReasonerError::ValidationError(_) => StatusCode::BAD_REQUEST,
                ReasonerError::ApiError(_) => StatusCode::BAD_GATEWAY,
                ReasonerError::ParseError(_) => StatusCode::BAD_GATEWAY,
                ReasonerError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                ReasonerError::NetworkError(_) => StatusCode::BAD_GATEWAY,
            };
            actix_web::HttpResponse::build(status).finish()
        }
    }
}
