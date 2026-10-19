use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "vikingthe.dev API"),
    paths(
        crate::health::health,
        crate::sessions::logout,
        crate::users::me,
        crate::users::session
    )
)]
pub struct ApiDoc;
