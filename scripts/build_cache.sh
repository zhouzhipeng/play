#!/usr/bin/env bash

docker  build -t zhouzhipeng/play-cache -f cache.Dockerfile .
docker push zhouzhipeng/play-cache