## Play
A playground project written in Rust for fun.

Features:
- General purpose data storage API with JSON support
- Plugin system for extensibility
- Modern web framework based on Axum
- Support for multiple storage backends

## Development

### Local Debug
The `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo debug
```

### Building from Source
```bash
# Clone the repository
git clone https://github.com/zhouzhipeng/play.git
cd play

# Build with default features
cargo build --release

# Build with debug features
cargo build --features debug
```

## Deployment

### Install as Linux Service
```bash
curl -sSL https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh | sudo bash
```

### Docker Support
The project includes Dockerfiles for containerized deployment.
```bash
# Build Docker image
docker build -t play .

# Run container
docker run -p 8080:8080 play
```

## Documentation

### Plugin Development
For details on creating plugins, see the [plugin development guide](docs/plugin-dev.md).

### Quick Development Guide
See the [quick development guide](docs/quick_dev.md) for getting started.


## General Data API Documentation
### API v4 (Latest)
* [English Doc](docs/api-v4-doc-en.md)
* [中文文档](docs/api-v4-doc-cn.md)


