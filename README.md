# Play
A high-performance general-purpose data storage server written in Rust, featuring a flexible JSON-based API with powerful querying capabilities.

## Features
- **RESTful Data API v4** - Store, query, and manage JSON data with SQL-like filtering
- **Flexible Storage** - Category-based data organization with automatic JSON field extraction
- **Advanced Querying** - Support for complex WHERE clauses, sorting, pagination, and field selection
- **Soft Delete Support** - Built-in soft delete functionality with optional hard delete
- **Plugin System** - Extensible architecture for custom functionality
- **Modern Framework** - Built on Axum for high performance and async support
- **Multiple Storage Backends** - Flexible database support


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
bash <(curl -Ls https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh)
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


## API Documentation

### General Data API v4 (Latest)
The Data API provides a powerful REST interface for storing and querying JSON data with SQL-like capabilities.

**Documentation:**
* [English Documentation](docs/api-v4-doc-en.md)
* [中文文档](docs/api-v4-doc-cn.md)

### JavaScript Client Library
A comprehensive JavaScript client library is available for easy integration with web applications.

**CDN URL:** `https://zhouzhipeng.com/static/js/data_api.js`

**Quick Start:**
```html
<script src="https://zhouzhipeng.com/static/js/data_api.js"></script>
<script>
// Create client instance
const client = new DataAPIClient('https://your-api-server.com');

// Example: Insert data
await client.insert('products', {
    name: 'Product 1',
    price: 99.99,
    active: true
});

// Example: Query with conditions
const products = await client.query('products', {
    where: 'price>50 AND active=true',
    order_by: 'price asc',
    limit: '0,10'
});
</script>
```

**NPM Installation:**
```bash
# Download the client
curl -O https://zhouzhipeng.com/static/js/data_api.js
```

**Features:**
- Full CRUD operations support (insert, get, query, update, delete)
- Advanced query builder with WHERE clause helpers
- Batch operations for parallel requests
- Built-in validation utilities
- TypeScript-compatible with JSDoc annotations

For detailed usage examples and API reference, see the [JavaScript client documentation](docs/api-v4-doc-en.md#javascript-client)


