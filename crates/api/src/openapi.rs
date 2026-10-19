use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(info(title = "vikingthe.dev API"), paths(crate::health::health))]
pub struct ApiDoc;
