from typing import Optional, List
from enum import Enum
import datetime
import os
import asyncio

from pydantic import BaseModel
from motor.motor_asyncio import AsyncIOMotorClient
from fastapi import FastAPI, HTTPException, BackgroundTasks
import uuid


MONGO_URI = os.getenv("MONGO_URI")
client = AsyncIOMotorClient(MONGO_URI)
db = client.sisyphus_fastapi
tasks_collection = db.tasks


class TaskType(str, Enum):
    cpu = "Cpu"
    memory = "Memory"
    io = "Io"

class TaskStatus(str, Enum):
    pending = "Pending"
    running = "Running"
    finished = "Finished"

class TaskParams(BaseModel):
    duration_millis: int
    memory_usage: Optional[int] = None

class Task(BaseModel):
    id: str
    type: TaskType
    blocking: bool
    params: TaskParams
    status: TaskStatus
    submitted_at: datetime.datetime
    started_at: Optional[datetime.datetime] = None
    finished_at: Optional[datetime.datetime] = None

    def dict(self):
        return {
            "id": self.id,
            "type": self.type.value,
            "blocking": self.blocking,
            "params": self.params.dict(),
            "status": self.status.value,
            "submitted_at": self.submitted_at.isoformat(),
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "finished_at": self.finished_at.isoformat() if self.finished_at else None
        }

class NewTask(BaseModel):
    type: TaskType
    blocking: bool
    params: TaskParams

class TasksList(BaseModel):
    tasks: List[Task]

class TasksStats(BaseModel):
    total: int = 0
    pending: int = 0
    running: int = 0
    finished: int = 0
    avg_runtime_millis: float = 0.0
    avg_e2e_time_millis: float = 0.0
    avg_wait_time_millis: float = 0.0
    types: dict = {}


async def cpu_bound_task(duration_millis: int):
    end_time = asyncio.get_running_loop().time() + duration_millis
    while asyncio.get_running_loop().time() < end_time:
        _ = [n for n in range(2, 10000) if all(n % i != 0 for i in range(2, int(n**0.5) + 1))]

async def memory_bound_task(memory_usage: int, duration_millis: int):
    _memory_hog = bytearray(memory_usage)
    await asyncio.sleep(duration_millis)

async def io_bound_task(duration_millis: int):
    await asyncio.sleep(duration_millis)


async def execute_task(task: Task):
    task.started_at = datetime.datetime.now(datetime.timezone.utc)
    task.status = TaskStatus.running
    await tasks_collection.update_one(
        {"id": task.id},
        {"$set": {"status": task.status, "started_at": task.started_at.isoformat()}}
    )

    if task.type == TaskType.cpu:
        await cpu_bound_task(task.params.duration_millis)
    elif task.type == TaskType.memory:
        await memory_bound_task(task.params.memory_usage or 1024 * 1024, task.params.duration_millis)
    elif task.type == TaskType.io:
        await io_bound_task(task.params.duration_millis)

    task.finished_at = datetime.datetime.now(datetime.timezone.utc)
    task.status = TaskStatus.finished
    await tasks_collection.update_one(
        {"id": task.id},
        {"$set": {"status": task.status, "finished_at": task.finished_at.isoformat()}}
    )


app = FastAPI()


@app.post("/tasks", response_model=Task)
async def submit_task(new_task: NewTask, background_tasks: BackgroundTasks):
    task = Task(
        id=str(uuid.uuid4()),
        type=new_task.type,
        blocking=new_task.blocking,
        params=new_task.params,
        status=TaskStatus.pending,
        submitted_at=datetime.datetime.now(datetime.timezone.utc)
    )

    result = await tasks_collection.insert_one(task.dict())
    if not result.acknowledged:
        raise HTTPException(status_code=500, detail="Task creation failed")

    if task.blocking:
        await execute_task(task)
    else:
        background_tasks.add_task(execute_task, task)

    return task

@app.get("/tasks", response_model=TasksList)
async def list_tasks():
    cursor = tasks_collection.find()
    tasks = await cursor.to_list(None)
    return TasksList(tasks=tasks)

@app.get("/tasks/{task_id}", response_model=Task)
async def get_task(task_id: str):
    task = await tasks_collection.find_one({"id": task_id})
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")
    return task

@app.get("/taskstats", response_model=TasksStats)
async def get_task_stats():
    cursor = tasks_collection.find()
    tasks = await cursor.to_list(None)
    stats = TasksStats(total=len(tasks))
    runtime_sum, e2e_sum, wait_sum = 0, 0, 0

    for task in tasks:
        task = Task(**task)
        stats.types[task.type.value] = stats.types.get(task.type.value, 0) + 1
        if task.status == TaskStatus.pending:
            stats.pending += 1
        elif task.status == TaskStatus.running:
            stats.running += 1
        elif task.status == TaskStatus.finished:
            stats.finished += 1
            if task.started_at and task.finished_at:
                runtime = (task.finished_at - task.started_at).total_seconds() * 1000
                e2e = (task.finished_at - task.submitted_at).total_seconds() * 1000
                wait = (task.started_at - task.submitted_at).total_seconds() * 1000
                runtime_sum += runtime
                e2e_sum += e2e
                wait_sum += wait
                stats.avg_runtime_millis = runtime_sum / stats.finished
                stats.avg_e2e_time_millis = e2e_sum / stats.finished
                stats.avg_wait_time_millis = wait_sum / stats.finished

    return stats


if __name__ == "__main__":
    import uvicorn
    import sys

    host = str(sys.argv[1]) if len(sys.argv) > 1 else "0.0.0.0"
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 8081

    print(f"Starting server at http://{host}:{port}")
    uvicorn.run(app, host=host, port=port)