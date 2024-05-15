use crate::models::{Task, TaskParams, TaskType};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewTask {
    #[serde(rename = "type")]
    pub ty: TaskType,
    pub blocking: bool,
    pub params: TaskParams,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TasksList {
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Default, ToSchema)]
pub struct TasksStats {
    pub total: usize,
    pub running: usize,
    pub pending: usize,
    pub finished: usize,
    pub types: BTreeMap<String, usize>,

    pub avg_runtime_millis: f64,
    pub avg_e2e_time_millis: f64,
    pub avg_wait_time_millis: f64,
}
