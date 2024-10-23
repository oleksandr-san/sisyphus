use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Clone, strum_macros::Display, ToSchema, Debug)]
pub enum TaskType {
    Cpu,
    Memory,
    Io,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct TaskParams {
    pub duration_millis: u64,
    pub memory_usage: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, strum_macros::Display, ToSchema, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Finished,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct Task {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: TaskType,
    pub blocking: bool,
    pub params: TaskParams,
    pub status: TaskStatus,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub result: Option<u64>,
}

impl Task {
    pub fn runtime(&self) -> Option<Duration> {
        match (self.started_at, self.finished_at) {
            (Some(started_at), Some(finished_at)) => Some(finished_at - started_at),
            _ => None,
        }
    }

    pub fn e2e_time(&self) -> Option<Duration> {
        match (self.submitted_at, self.finished_at) {
            (submitted_at, Some(finished_at)) => Some(finished_at - submitted_at),
            _ => None,
        }
    }

    pub fn wait_time(&self) -> Option<Duration> {
        match (self.started_at, self.submitted_at) {
            (Some(started_at), submitted_at) => Some(started_at - submitted_at),
            _ => None,
        }
    }
}
