# Makefile for building C/C++ plugins
# Usage:
#   make c_plugin    - Build C plugin
#   make cpp_plugin  - Build C++ plugin
#   make all         - Build both plugins
#   make clean       - Clean build artifacts

# Detect OS
UNAME_S := $(shell uname -s)

# Compiler settings
CC = gcc
CXX = g++
CFLAGS = -Wall -O2 -fPIC
CXXFLAGS = -Wall -O2 -fPIC -std=c++17

# Library dependencies
LIBS_C = -lcurl -ljson-c
LIBS_CPP = -lcurl -ljsoncpp

# Output file extension based on OS
ifeq ($(UNAME_S),Linux)
    EXT = .so
    LDFLAGS = -shared
endif
ifeq ($(UNAME_S),Darwin)
    EXT = .dylib
    LDFLAGS = -shared
endif

# Default target
all: c_plugin cpp_plugin

# Build C plugin
c_plugin: example_plugin.c plugin_abi.h
	@echo "Building C plugin..."
	$(CC) $(CFLAGS) $(LDFLAGS) -o c_plugin$(EXT) example_plugin.c $(LIBS_C)
	@echo "Built: c_plugin$(EXT)"

# Build C++ plugin
cpp_plugin: example_plugin.cpp plugin_abi.h
	@echo "Building C++ plugin..."
	$(CXX) $(CXXFLAGS) $(LDFLAGS) -o cpp_plugin$(EXT) example_plugin.cpp $(LIBS_CPP)
	@echo "Built: cpp_plugin$(EXT)"

# Clean build artifacts
clean:
	@echo "Cleaning..."
	rm -f c_plugin$(EXT) cpp_plugin$(EXT)
	@echo "Clean complete"

# Install dependencies (example for Ubuntu/Debian)
install-deps:
	@echo "Installing dependencies (requires sudo)..."
	@if [ "$(UNAME_S)" = "Linux" ]; then \
		sudo apt-get update && \
		sudo apt-get install -y libcurl4-openssl-dev libjson-c-dev libjsoncpp-dev; \
	elif [ "$(UNAME_S)" = "Darwin" ]; then \
		brew install curl json-c jsoncpp; \
	fi

# Test plugins (requires running Play server)
test: all
	@echo "Testing plugins requires a running Play server at HOST=http://127.0.0.1:3000"
	@echo "Configure the plugins in your config.toml:"
	@echo "  [[plugin_config]]"
	@echo "  name = \"c_plugin\""
	@echo "  file_path = \"$(PWD)/c_plugin$(EXT)\""
	@echo "  url_prefix = \"/c-plugin\""
	@echo ""
	@echo "  [[plugin_config]]"
	@echo "  name = \"cpp_plugin\""
	@echo "  file_path = \"$(PWD)/cpp_plugin$(EXT)\""
	@echo "  url_prefix = \"/cpp-plugin\""

.PHONY: all c_plugin cpp_plugin clean install-deps test