import asyncio
import httpx
import time
import os
from statistics import mean

API_URL = os.getenv("API_URL", "http://localhost:8083")  # Change to your API URL
CONCURRENT_REQUESTS = int(os.getenv("CONCURRENT_REQUESTS", 100))
INTERVAL = int(os.getenv("INTERVAL", 10))
TASK_DURATION_MILLIS = int(os.getenv("TASK_DURATION_MILLIS", 100))
TASK_TYPE = str(os.getenv("TASK_TYPE", "Io"))
TASK_MEMORY_USAGE = int(os.getenv("TASK_MEMORY_USAGE", 1024**3))
TIMEOUT = int(os.getenv("TIMEOUT", 60))
CONNECT_TIMEOUT = int(os.getenv("CONNECT_TIMEOUT", 10))

async def run_task(client, url):
    task_data = {
        "type": TASK_TYPE,
        "blocking": False,
        "params": {
            "duration_millis": TASK_DURATION_MILLIS,
            "memory_usage": TASK_MEMORY_USAGE
        }
    }
    start_time = time.time()
    response = await client.post(f"{url}/tasks", json=task_data)
    end_time = time.time()
    return response, end_time - start_time


async def measure_rps(client, url, duration):
    start_time = time.time()
    end_time = start_time + duration
    request_count = 0
    failed_requests = 0
    response_times = []

    async def make_request():
        nonlocal request_count, failed_requests
        while time.time() < end_time:
            try:
                response, elapsed_time = await run_task(client, url)
                if response.status_code == 200:
                    request_count += 1
                    response_times.append(elapsed_time)
                else:
                    failed_requests += 1
            except Exception as e:
                print(f"Request failed: {e}")
                failed_requests += 1

    tasks = [make_request() for _ in range(CONCURRENT_REQUESTS)]
    await asyncio.gather(*tasks)

    total_time = time.time() - start_time
    rps = request_count / total_time
    avg_response_time = mean(response_times) if response_times else 0
    max_response_time = max(response_times, default=0)
    min_response_time = min(response_times, default=0)

    print(f"Requests per second: {rps:.2f}")
    print(f"Total Requests: {request_count}")
    print(f"Failed Requests: {failed_requests}")
    print(f"Average Request Time: {avg_response_time:.2f} seconds")
    print(f"Max Request Time: {max_response_time:.2f} seconds")
    print(f"Min Request Time: {min_response_time:.2f} seconds")
    print("-" * 40)


async def main():
    print(f"Running benchmark on {API_URL}")
    timeout = httpx.Timeout(TIMEOUT, connect=CONNECT_TIMEOUT)  # Increase timeout settings
    async with httpx.AsyncClient(timeout=timeout) as client:
        while True:
            await measure_rps(client, API_URL, INTERVAL)

if __name__ == "__main__":
    asyncio.run(main())

