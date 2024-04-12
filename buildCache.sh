#!/usr/bin/env bash

# Function to check if a specific builder exists
builder_exists() {
    docker buildx ls | grep -q $1
}

# Builder name
BUILDER_NAME="mybuilder"

# Check if the builder already exists
if builder_exists $BUILDER_NAME; then
    echo "Builder $BUILDER_NAME already exists, switching to it."
    docker buildx use $BUILDER_NAME
else
    echo "Creating and using new builder $BUILDER_NAME."
    docker buildx create --name $BUILDER_NAME --use
fi

docker buildx build --platform linux/amd64,linux/arm64 -t zhouzhipeng/play-cache -f cache.Dockerfile --push .
