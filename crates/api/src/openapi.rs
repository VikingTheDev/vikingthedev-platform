use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "vikingthe.dev API"),
    paths(crate::health::health, crate::sessions::logout)
)]
pub struct ApiDoc;
