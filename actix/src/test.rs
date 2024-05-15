use actix_web::{
    test::{call_and_read_body, call_and_read_body_json, init_service, TestRequest},
    web::Bytes,
};

use super::*;

async fn clean_db(client: &Client) {
    client
        .database(db::DB_NAME)
        .collection::<models::User>(db::USERS_COLL_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");

    client
        .database(db::DB_NAME)
        .collection::<models::Task>(db::TASKS_COLL_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");
}

async fn setup_db() -> Client {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let client = Client::with_uri_str(uri).await.expect("failed to connect");

    clean_db(&client).await;
    client
}

#[actix_web::test]
async fn tasks_test() {
    let client = setup_db().await;
    let app = init_service(App::new().configure(handlers::configure(client))).await;

    let new_task = schemas::NewTask {
        ty: models::TaskType::Cpu,
        blocking: true,
        params: models::TaskParams {
            duration_millis: 1000,
            memory_usage: None,
        },
    };

    let req = TestRequest::post()
        .uri("/tasks")
        .set_json(&new_task)
        .to_request();
    let task: models::Task = call_and_read_body_json(&app, req).await;

    let req = TestRequest::get()
        .uri(&format!("/tasks/{}", &task.id))
        .to_request();

    let response: models::Task = call_and_read_body_json(&app, req).await;
    assert_eq!(response.id, task.id);
    assert_eq!(response.status, models::TaskStatus::Finished);

    let req = TestRequest::get().uri("/tasks").to_request();
    let response: schemas::TasksList = call_and_read_body_json(&app, req).await;
    assert_eq!(response.tasks.len(), 1);

    let req = TestRequest::get().uri("/taskstats").to_request();
    let response: schemas::TasksStats = call_and_read_body_json(&app, req).await;
    assert_eq!(response.total, 1);
    assert_eq!(response.pending, 0);
    assert_eq!(response.running, 0);
    assert_eq!(response.finished, 1);
    assert!(response.avg_runtime_millis > 1000.0);
}

#[actix_web::test]
async fn users_test() {
    let client = setup_db().await;
    let app = init_service(App::new().configure(handlers::configure(client))).await;

    let user = models::User {
        first_name: "Jane".into(),
        last_name: "Doe".into(),
        username: "janedoe".into(),
        email: "example@example.com".into(),
    };

    let req = TestRequest::post()
        .uri("/users")
        .set_form(&user)
        .to_request();

    let response = call_and_read_body(&app, req).await;
    assert_eq!(response, Bytes::from_static(b"user added"));

    let req = TestRequest::get()
        .uri(&format!("/users/{}", &user.username))
        .to_request();

    let response: models::User = call_and_read_body_json(&app, req).await;
    assert_eq!(response, user);
}