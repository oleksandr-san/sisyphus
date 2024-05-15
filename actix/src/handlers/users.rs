use crate::db::{DB_NAME, USERS_COLL_NAME};
use crate::models::User;

use actix_web::{get, post, web, HttpResponse};
use mongodb::{bson::doc, Client, Collection};

#[utoipa::path(
    responses(
        (status = 200, description = "Add user", body = User),
    ),
)]
#[post("/users")]
async fn add(client: web::Data<Client>, form: web::Form<User>) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(USERS_COLL_NAME);
    let result = collection.insert_one(form.into_inner(), None).await;
    match result {
        Ok(_) => HttpResponse::Ok().body("user added"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[utoipa::path(
    responses(
        (status = 200, description = "Get user", body = User),
    ),
    params(
        ("username", description = "User name", example = "me")
    )
)]
#[get("/users/{username}")]
async fn get(client: web::Data<Client>, username: web::Path<String>) -> HttpResponse {
    let username = username.into_inner();
    let collection: Collection<User> = client.database(DB_NAME).collection(USERS_COLL_NAME);
    match collection
        .find_one(doc! { "username": &username }, None)
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => {
            HttpResponse::NotFound().body(format!("No user found with username {username}"))
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
