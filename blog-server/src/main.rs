//! Точка входа: связывает HTTP (Actix) и gRPC (Tonic) с одними и теми же сервисами приложения.
mod application;
mod config;
mod data;
mod domain;
mod infrastructure;
mod presentation;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use tonic::transport::Server;

use presentation::grpc_service::pb::blog_service_server::BlogServiceServer;
use presentation::grpc_service::BlogGrpcService;
use presentation::http_handlers::{
    create_post, delete_post, get_post, list_posts, login, register, update_post,
};
use presentation::middleware::jwt_validator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Сначала `blog-server/.env` при запуске из корня workspace, иначе `.env` из текущей директории.
    dotenvy::from_filename(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env")).ok();
    dotenvy::dotenv().ok();
    infrastructure::logging::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = infrastructure::database::create_pool(&database_url).await?;
    // Встроенные миграции из `blog-server/migrations/` (см. макрос `sqlx::migrate!`).
    infrastructure::database::run_migrations(&pool).await?;

    let jwt = Arc::new(infrastructure::jwt::JwtService::new(&jwt_secret));
    let users = Arc::new(data::user_repository::PostgresUserRepository::new(pool.clone()));
    let posts = Arc::new(data::post_repository::PostgresPostRepository::new(pool));

    let auth = Arc::new(application::auth_service::AuthService::new(users, jwt.clone()));
    let blog = Arc::new(application::blog_service::BlogService::new(posts));

    let grpc_service = BlogGrpcService {
        auth: auth.clone(),
        blog: blog.clone(),
        jwt: jwt.clone(),
    };

    let auth_data = web::Data::new(auth.clone());
    let blog_data = web::Data::new(blog.clone());
    let jwt_data = web::Data::new(jwt);

    let http_server = HttpServer::new(move || {
        // Для браузера/WASM в разработке — открытый CORS; в продакшене ограничьте origins.
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allowed_methods(vec![
                actix_web::http::Method::GET,
                actix_web::http::Method::POST,
                actix_web::http::Method::PUT,
                actix_web::http::Method::DELETE,
                actix_web::http::Method::OPTIONS,
            ])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(auth_data.clone())
            .app_data(blog_data.clone())
            .app_data(jwt_data.clone())
            .service(
                web::scope("/api")
                    .route("/auth/register", web::post().to(register))
                    .route("/auth/login", web::post().to(login))
                    // Публичное чтение: список и пост по id (без JWT).
                    .service(
                        web::scope("/posts")
                            .route("", web::get().to(list_posts))
                            .route("/{id}", web::get().to(get_post)),
                    )
                    // Изменение данных только с заголовком `Authorization: Bearer <jwt>` (см. `jwt_validator`).
                    .service(
                        web::scope("/posts")
                            .wrap(HttpAuthentication::bearer(jwt_validator))
                            .route("", web::post().to(create_post))
                            .route("/{id}", web::put().to(update_post))
                            .route("/{id}", web::delete().to(delete_post)),
                    ),
            )
    })
    .bind(config::HTTP_BIND)?
    .run();

    let addr = config::GRPC_BIND.parse()?;
    let grpc_server = Server::builder()
        .add_service(BlogServiceServer::new(grpc_service))
        .serve(addr);

    tracing::info!("HTTP listening on {}", config::HTTP_BIND);
    tracing::info!("gRPC listening on {}", config::GRPC_BIND);

    // Оба сервера на одном runtime Tokio; выход при завершении или ошибке любой задачи.
    tokio::select! {
        res = http_server => res.map_err(anyhow::Error::from),
        res = grpc_server => res.map_err(anyhow::Error::from),
    }
}
