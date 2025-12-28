use std::sync::Arc;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{App, Error, web};

use crate::{config, handlers, service};

pub fn create_app(
    reasoning_service: Arc<service::ReasoningService>,
    config: Arc<config::Config>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    App::new()
        .wrap(Logger::default())
        .app_data(Data::from(reasoning_service))
        .app_data(Data::from(config))
        .service(
            web::scope("/v1")
                .route("/models", web::get().to(handlers::models))
                .route(
                    "/chat/completions",
                    web::post().to(handlers::chat_completion),
                ),
        )
}
