pub mod admin;
pub mod shares;
pub mod tables;
use crate::config;
use crate::server::api_doc::ApiDoc;
use crate::server::middlewares::jwt;
use crate::server::services::error::Error;
use anyhow::Context;
use anyhow::Result;
use axum::extract::Extension;
use axum::http::Uri;
use axum::middleware;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use rusoto_credential::ProfileProvider;
use sqlx::PgPool;
use std::sync::Arc;
use tame_gcs::signing::ServiceAccount;
use tracing::debug;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub struct State {
    pub pg_pool: PgPool,
    pub gcp_service_account: ServiceAccount,
    pub aws_profile_provider: ProfileProvider,
}

pub type SharedState = Arc<State>;

async fn bad_request(_: Uri) -> std::result::Result<Response, Error> {
    Err(Error::BadRequest)
}

async fn route(
    pg_pool: PgPool,
    gcp_service_account: ServiceAccount,
    aws_profile_provider: ProfileProvider,
) -> Result<Router> {
    let state = Arc::new(State {
        pg_pool,
        gcp_service_account,
        aws_profile_provider,
    });

    let swagger = SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi());

    let admin = Router::new()
        .route("/admin/accounts", post(self::admin::accounts::post))
        .route("/admin/accounts", get(self::admin::accounts::list))
        .route("/admin/accounts/:account", get(self::admin::accounts::get))
        .route("/admin/shares", post(self::admin::shares::post))
        .route("/admin/tables", post(self::admin::tables::post))
        .route("/admin/tables", get(self::admin::tables::list))
        .route("/admin/tables/:table", get(self::admin::tables::get))
        .route(
            "/admin/shares/:share/schemas/:schema/tables",
            post(admin::shares::schemas::tables::post),
        )
        .route_layer(middleware::from_fn(jwt::as_admin))
        .route("/admin/login", post(self::admin::login))
        .layer(Extension(state.clone()));

    let guest = Router::new()
        .route("/shares", get(self::shares::list))
        .route("/shares/:share", get(self::shares::get))
        .route("/shares/:share/schemas", get(self::shares::schemas::list))
        .route(
            "/shares/:share/schemas/:schema/tables",
            get(self::shares::schemas::tables::list),
        )
        .route_layer(middleware::from_fn(jwt::as_guest))
        .layer(Extension(state.clone()));

    let app = Router::new()
        .merge(swagger)
        .merge(admin)
        .merge(guest)
        .fallback(bad_request);

    Ok(app)
}

pub async fn bind(
    pg_pool: PgPool,
    gcp_service_account: ServiceAccount,
    aws_profile_provider: ProfileProvider,
) -> Result<()> {
    let app = route(pg_pool, gcp_service_account, aws_profile_provider)
        .await
        .context("failed to create axum router")?;
    let server_bind = config::fetch::<String>("server_bind");
    let addr = server_bind.as_str().parse().context(format!(
        r#"failed to parse "{}" to SocketAddr"#,
        server_bind
    ))?;
    debug!("kotosiro sharing server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context(format!(
            r#"failed to bind "{}" to hyper::Server"#,
            server_bind,
        ))?;
    Ok(())
}
