use crate::models::{Task, User};

use mongodb::{bson::doc, options::IndexOptions, Client, IndexModel};

pub const DB_NAME: &str = "myApp";
pub const USERS_COLL_NAME: &str = "users";
pub const TASKS_COLL_NAME: &str = "tasks";

pub async fn create_username_index(client: &Client) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "username": 1 })
        .options(options)
        .build();
    client
        .database(DB_NAME)
        .collection::<User>(USERS_COLL_NAME)
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}

pub async fn create_task_indices(client: &Client) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "id": 1 })
        .options(options)
        .build();
    client
        .database(DB_NAME)
        .collection::<Task>(TASKS_COLL_NAME)
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}
