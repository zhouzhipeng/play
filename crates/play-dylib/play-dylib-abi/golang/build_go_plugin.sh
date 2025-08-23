#!/bin/bash

# Build script for Go plugin using new architecture

echo "Building Go plugin with new architecture..."

# Combine golang_abi.go and example_plugin.go into a single file for building
cat golang_abi.go > combined_plugin.go
echo "" >> combined_plugin.go
cat example_plugin.go | grep -v "^package main" | grep -v "^import" >> combined_plugin.go

# Build the shared library
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    go build -buildmode=c-shared -o libexample_plugin.dylib combined_plugin.go
    echo "Built libexample_plugin.dylib for macOS"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    go build -buildmode=c-shared -o libexample_plugin.so combined_plugin.go
    echo "Built libexample_plugin.so for Linux"
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

# Clean up temporary file
rm -f combined_plugin.go

echo "Build complete!"
echo ""
echo "To use the plugin:"
echo "1. Set the HOST environment variable: export HOST=http://127.0.0.1:3000"
echo "2. Configure the plugin in your config.toml"
echo "3. The plugin will handle requests using the new request_id pattern"