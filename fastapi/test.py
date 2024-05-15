import pytest
from httpx import AsyncClient
from motor.motor_asyncio import AsyncIOMotorClient
from app import app as fastapi_app  # Import your FastAPI app
from app import TaskType, TaskStatus, NewTask, TaskParams, TasksList, TasksStats

MONGODB_URI = "mongodb://localhost:27017"

@pytest.fixture(scope="module")
async def client():
    async with AsyncClient(app=fastapi_app, base_url="http://test") as client:
        yield client

@pytest.fixture(scope="module")
async def mongo_client():
    client = AsyncIOMotorClient(MONGODB_URI)
    yield client
    client.close()

@pytest.mark.asyncio
async def test_tasks(client: AsyncClient, mongo_client: AsyncIOMotorClient):
    # Ensure the database is clean

    db = mongo_client["sisyphus_fastapi"]
    await db["tasks"].drop()

    # Submit a new task
    new_task = NewTask(
        type=TaskType.cpu,
        blocking=True,
        params=TaskParams(
            duration_millis=1,
            memory_usage=None
        )
    )
    
    response = await client.post("/tasks", json=new_task.dict())
    assert response.status_code == 200
    task = response.json()
    task_id = task['id']

    # Get the task by ID
    response = await client.get(f"/tasks/{task_id}")
    assert response.status_code == 200
    fetched_task = response.json()
    assert fetched_task['id'] == task_id
    assert fetched_task['status'] == TaskStatus.finished

    # List all tasks
    response = await client.get("/tasks")
    assert response.status_code == 200
    tasks_list = TasksList(**response.json())
    assert len(tasks_list.tasks) == 1

    # Get task stats
    response = await client.get("/taskstats")
    assert response.status_code == 200
    stats = TasksStats(**response.json())
    assert stats.total == 1
    assert stats.pending == 0
    assert stats.running == 0
    assert stats.finished == 1
    assert stats.avg_runtime_millis > 1000.0