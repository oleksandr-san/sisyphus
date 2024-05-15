pub mod tasks;
pub mod users;

use actix_web::web::{self, ServiceConfig};
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        users::get,
        users::add,
        tasks::get,
        tasks::submit,
        tasks::list,
        tasks::stats,
    ),
    components(
        schemas(
            crate::schemas::NewTask,
            crate::schemas::TasksStats,
            crate::schemas::TasksList,
            crate::models::TaskParams,
            crate::models::TaskStatus,
            crate::models::TaskType,
            crate::models::Task,
            crate::models::User,
        )
    ),
    tags(
        (name = "Tasks", description = "Simple DEMO service.")
    )
)]
pub(super) struct ApiDoc;

pub(super) fn configure(client: mongodb::Client) -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        let openapi = ApiDoc::openapi();

        config
            .app_data(web::Data::new(client))
            .service(users::get)
            .service(users::add)
            .service(tasks::get)
            .service(tasks::submit)
            .service(tasks::list)
            .service(tasks::stats)
            .service(Redoc::with_url("/redoc", openapi.clone()))
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            );
    }
}
