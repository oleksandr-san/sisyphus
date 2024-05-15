FROM public.ecr.aws/docker/library/python:3.11

RUN apt-get update && \
    apt-get install --yes --no-install-recommends libjemalloc2
ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.2

WORKDIR /usr/src/app

# optimizing
ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

# install dependencies
RUN pip install --upgrade pip

COPY requirements.txt /usr/src/app/
RUN pip install --no-cache-dir -r ./requirements.txt

COPY bench.py /usr/src/app/
ENTRYPOINT [ "python", "bench.py" ]
