use crate::db::{DB_NAME, TASKS_COLL_NAME};
use crate::models::{Task, TaskStatus, TaskType};
use crate::schemas::{NewTask, TasksList, TasksStats};

use actix_web::{get, post, web, HttpResponse};
use futures::stream::StreamExt;
use mongodb::bson::Uuid;
use mongodb::{bson::doc, Client, Collection};
use tokio::time::{sleep, Duration};
use tracing::{debug, info};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const DEFAULT_MEMORY_USAGE: usize = 1024 * 1024;

async fn cpu_bound_task(duration_millis: u64) -> u64 {
    let end = std::time::Instant::now() + Duration::from_millis(duration_millis);
    let mut result = 0;
    while std::time::Instant::now() < end {
        result = (2..10_000)
            .filter(|n| (2..(*n as f64).sqrt() as u64 + 1).all(|i| n % i != 0))
            .collect::<Vec<_>>()
            .iter()
            .sum();
    }
    result
}

async fn memory_bound_task(memory_usage: usize, duration_millis: u64) -> u64 {
    let mut memory_hog = vec![0u8; memory_usage];

    let mut rng = StdRng::from_entropy();
    for byte in memory_hog.iter_mut() {
        *byte = rng.gen();  // Fill with a random u8 value
    }

    sleep(Duration::from_millis(duration_millis)).await;

    let result: u64 = memory_hog.iter().map(|&x| x as u64).sum();
    result
}

async fn io_bound_task(duration_millis: u64) -> u64 {
    sleep(Duration::from_millis(duration_millis)).await;
    let mut rng = StdRng::from_entropy();
    rng.gen()
}

async fn execute_task(mut task: Task, client: Client) -> mongodb::error::Result<Task> {
    let collection: Collection<Task> = client.database(DB_NAME).collection(TASKS_COLL_NAME);

    info!("Start executing task {}: {}", task.id, task.ty);
    let started_at = chrono::Utc::now();
    task.started_at = Some(started_at);
    task.status = TaskStatus::Running;

    debug!("Update started task {:?} in database", task);
    collection
        .update_one(
            doc! { "id": &task.id },
            doc! { "$set": doc! {
                "status": task.status.to_string(),
                "started_at": started_at.to_rfc3339()
            } },
            None,
        )
        .await
        .expect("Error updating task");

    let result = match task.ty {
        TaskType::Cpu => cpu_bound_task(task.params.duration_millis).await,
        TaskType::Memory => {
            memory_bound_task(
                task.params.memory_usage.unwrap_or(DEFAULT_MEMORY_USAGE),
                task.params.duration_millis,
            )
            .await
        }
        TaskType::Io => io_bound_task(task.params.duration_millis).await,
    };

    let finished_at = chrono::Utc::now();
    task.finished_at = Some(finished_at);
    task.status = TaskStatus::Finished;
    task.result = Some(result);

    debug!("Update finished task {:?} in database", task);
    collection
        .update_one(
            doc! { "id": &task.id },
            doc! { "$set": doc! {
                "status": task.status.to_string(),
                "finished_at": finished_at.to_rfc3339(),
            } },
            None,
        )
        .await?;

    info!("Finish executing task {}: {}", task.id, task.ty);
    Ok(task)
}

#[utoipa::path(
    request_body = NewTask,
    responses(
        (status = 200, description = "Task created"),
    )
)]
#[post("/tasks")]
async fn submit(
    client: web::Data<Client>,
    web::Json(new_task): web::Json<NewTask>,
) -> HttpResponse {
    let collection: Collection<Task> = client.database(DB_NAME).collection(TASKS_COLL_NAME);
    let task = Task {
        id: Uuid::new().to_string(),
        ty: new_task.ty,
        blocking: new_task.blocking,
        params: new_task.params,
        status: TaskStatus::Pending,
        submitted_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        result: None,
    };

    debug!("Store new taks in database: {}", task.id);
    if let Err(err) = collection.insert_one(&task, None).await {
        return HttpResponse::InternalServerError().body(err.to_string());
    }

    if task.blocking {
        debug!("Execute blocking taks: {}", task.id);
        match execute_task(task, client.get_ref().clone()).await {
            Ok(task) => HttpResponse::Ok().json(task),
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        }
    } else {
        debug!("Spawn non-blocking taks: {}", task.id);
        let sent_task = task.clone();
        tokio::spawn(async move {
            if let Err(err) = execute_task(sent_task, client.get_ref().clone()).await {
                eprintln!("Error executing task: {}", err);
            }
        });
        HttpResponse::Ok().json(task)
    }
}

#[utoipa::path(
    responses(
        (status = 200, description = "Tasks list", body = TasksList),
    )
)]
#[get("/tasks")]
async fn list(client: web::Data<Client>) -> HttpResponse {
    let collection: Collection<Task> = client.database(DB_NAME).collection(TASKS_COLL_NAME);

    let mut cursor = match collection.find(None, None).await {
        Ok(cursor) => cursor,
        Err(_) => return HttpResponse::InternalServerError().body("Error retrieving tasks"),
    };

    let mut response = TasksList { tasks: vec![] };
    while let Some(result) = cursor.next().await {
        match result {
            Ok(task) => {
                response.tasks.push(task);
            }
            Err(_) => return HttpResponse::InternalServerError().body("Error retrieving tasks"),
        }
    }
    HttpResponse::Ok().json(response)
}

#[utoipa::path(
    responses(
        (status = 200, description = "Get task", body = Task),
    ),
    params(
        ("id", description = "Task id", example = "me")
    )
)]
#[get("/tasks/{id}")]
async fn get(client: web::Data<Client>, id: web::Path<String>) -> HttpResponse {
    let id: String = id.into_inner();
    let collection: Collection<Task> = client.database(DB_NAME).collection(TASKS_COLL_NAME);

    match collection.find_one(doc! { "id": &id }, None).await {
        Ok(Some(task)) => HttpResponse::Ok().json(task),
        Ok(None) => HttpResponse::NotFound().body(format!("No task found with id {id}")),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[utoipa::path(
    responses(
        (status = 200, description = "Tasks stats", body = TasksStats),
    )
)]
#[get("/taskstats")]
async fn stats(client: web::Data<Client>) -> HttpResponse {
    let collection: Collection<Task> = client.database(DB_NAME).collection(TASKS_COLL_NAME);

    let mut cursor = match collection.find(None, None).await {
        Ok(cursor) => cursor,
        Err(_) => return HttpResponse::InternalServerError().body("Error retrieving tasks"),
    };

    let mut stats = TasksStats::default();
    let mut runtime_val_count = 0;
    let mut e2e_val_count = 0;
    let mut wait_val_count = 0;

    while let Some(result) = cursor.next().await {
        match result {
            Ok(task) => {
                match task.status {
                    TaskStatus::Pending => stats.pending += 1,
                    TaskStatus::Running => stats.running += 1,
                    TaskStatus::Finished => stats.finished += 1,
                }
                stats.total += 1;

                let ty = task.ty.to_string();
                *stats.types.entry(ty).or_insert(0) += 1;

                if let Some(runtime) = task.runtime() {
                    runtime_val_count += 1;
                    stats.avg_runtime_millis += runtime.num_milliseconds() as f64;
                }

                if let Some(e2e_time) = task.e2e_time() {
                    e2e_val_count += 1;
                    stats.avg_e2e_time_millis += e2e_time.num_milliseconds() as f64;
                }

                if let Some(wait_time) = task.wait_time() {
                    wait_val_count += 1;
                    stats.avg_wait_time_millis += wait_time.num_milliseconds() as f64;
                }
            }
            Err(_) => return HttpResponse::InternalServerError().body("Error retrieving tasks"),
        }
    }

    if runtime_val_count > 0 {
        stats.avg_runtime_millis /= runtime_val_count as f64;
    }
    if e2e_val_count > 0 {
        stats.avg_e2e_time_millis /= e2e_val_count as f64;
    }
    if wait_val_count > 0 {
        stats.avg_wait_time_millis /= wait_val_count as f64;
    }

    HttpResponse::Ok().json(stats)
}
