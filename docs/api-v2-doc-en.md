# General Data API Documentation

## Overview

This API provides a flexible data storage system where data is organized by categories. It allows for storing, retrieving, updating, and deleting JSON data with various filtering and query options.

## Base URL

```
/api/v2/data
```

## Endpoints

### Data Operations

- `GET /api/v2/data/:category/:action` - Perform query actions using query parameters
- `POST /api/v2/data/:category/:action` - Perform actions using JSON body
- `GET /api/v2/data/:category/:action/:hex` - Perform actions using hex-encoded parameters

## Action Types

The API supports four main action types:

1. `insert` - Create new data entries
2. `update` - Update existing data entries
3. `query` - Retrieve data entries
4. `delete` - Remove data entries (soft or hard delete)

## Parameters

### Query Parameters

```
{
  "id": Optional<u32>,          // Specific record ID
  "select": Optional<String>,   // Fields to select (comma-separated)
  "limit": u32,                 // Default: 10
  "where": Optional<String>,    // Filter condition
  "order_by": Optional<String>, // Sort order
  "less": bool,                 // Default: false - Return simplified data
  "count": bool                 // Default: false - Return count instead of data
}
```

### Update Parameters

```
{
  "id": u32,        // Record ID to update
  "set": String     // Update expression (format: "field1=value1,field2=value2")
}
```

### Delete Parameters

```
{
  "id": u32,             // Record ID to delete
  "hard_delete": bool    // Default: false - If true, permanently deletes the record
}
```

## Data Model

The API operates on the `GeneralData` model with the following structure:

```
{
  "id": u32,                  // Unique identifier
  "cat": String,              // Category name
  "data": String,             // JSON data stored as string
  "is_deleted": bool,         // Deletion flag
  "created": NaiveDateTime,   // Creation timestamp
  "updated": NaiveDateTime    // Last update timestamp
}
```

## Detailed Usage

### Insert Operation

To insert new data:

**Method:** `GET` or `POST`  
**URL:** `/api/v2/data/:category/insert`  
**Parameters:** JSON object containing the data to insert

Notes:
- System fields (`id`, `cat`, `is_deleted`, `created`, `updated`) cannot be included in insert operations
- Returns the inserted data with all fields including the generated ID

### Update Operation

To update existing data:

**Method:** `GET` or `POST`  
**URL:** `/api/v2/data/:category/update`  
**Parameters:**
```
{
  "id": 123,
  "set": "field1=value1,field2=value2"
}
```

Notes:
- The `set` parameter must follow the format `field1=value1,field2=value2`
- Returns the number of affected rows

### Query Operation

To query data:

**Method:** `GET` or `POST`  
**URL:** `/api/v2/data/:category/query`  
**Parameters:**
```
{
  "id": 123,              // Optional: Query specific ID
  "select": "field1,field2",  // Optional: Fields to select
  "limit": 20,            // Default: 10
  "where": "field1='value1' AND field2>100",  // Optional: Filter condition
  "order_by": "id desc",  // Optional: Default "id desc"
  "less": true,           // Optional: Return only data field content
  "count": false          // Optional: Return count instead of data
}
```

Notes:
- If `id` is provided, it returns a single record
- If no `id` is provided, it returns a list of records
- The `where` condition supports SQL-like syntax with AND operators
- The `less` parameter when true returns only the JSON data content
- The `count` parameter when true returns the count of matching records

### Delete Operation

To delete data:

**Method:** `GET` or `POST`  
**URL:** `/api/v2/data/:category/delete`  
**Parameters:**
```
{
  "id": 123,
  "hard_delete": false  // Optional: Default false
}
```

Notes:
- By default, performs a soft delete (sets is_deleted=1)
- If `hard_delete` is true, removes the record permanently
- Returns the number of affected rows

## Advanced Features

### Hex-Encoded Requests

For more complex queries or to avoid URL encoding issues:

**URL:** `/api/v2/data/:category/:action/:hex`

Where `:hex` is a hexadecimal-encoded JSON string of parameters.

Example:
```
/api/v2/data/users/query/7B226964223A313233...
```

The hex value is decoded as a JSON string containing the query parameters.

### JSON Field Extraction

The API provides two methods for handling the returned data:

1. **to_flat_map()**: Merges system fields and JSON data into a flat structure
2. **extract_data()**: Returns only the JSON data portion

When using the query API:
- If `less` is `false` (default), returns full data with system fields
- If `less` is `true`, returns only the JSON data content

### Field Selection

The `select` parameter allows retrieving only specific fields:

- `"*"` returns all fields (default)
- `"field1,field2,field3"` returns only specified fields
- Fields are extracted from the JSON data using `json_extract`
- Selected fields are returned in a structured format

## Examples

### Insert Example

```
POST /api/v2/data/users/insert
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30
}
```

Response:
```json
{
  "id": 1,
  "cat": "users",
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30,
  "is_deleted": false,
  "created": 1651234567890,
  "updated": 1651234567890
}
```

### Query Example

```
GET /api/v2/data/users/query?id=1
```

Response:
```json
{
  "id": 1,
  "cat": "users",
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30,
  "is_deleted": false,
  "created": 1651234567890,
  "updated": 1651234567890
}
```

### List Query Example

```
GET /api/v2/data/users/query?limit=10&where=age>25&order_by=name%20asc
```

Response:
```json
[
  {
    "id": 1,
    "cat": "users",
    "name": "John Doe",
    "email": "john@example.com",
    "age": 30,
    "is_deleted": false,
    "created": 1651234567890,
    "updated": 1651234567890
  },
  {
    "id": 2,
    "cat": "users",
    "name": "Jane Smith",
    "email": "jane@example.com",
    "age": 28,
    "is_deleted": false,
    "created": 1651234568890,
    "updated": 1651234568890
  }
]
```

### Update Example

```
POST /api/v2/data/users/update
Content-Type: application/json

{
  "id": 1,
  "set": "name=Jane Doe,age=31"
}
```

Response:
```
1
```

### Delete Example

```
POST /api/v2/data/users/delete
Content-Type: application/json

{
  "id": 1,
  "hard_delete": false
}
```

Response:
```
1
```

## Error Handling

The API returns appropriate error messages for different scenarios:

- Invalid category or action
- Invalid parameters
- Database errors
- Data not found
- Permission issues

Errors follow a consistent format with descriptive messages to help diagnose issues.

## Special Features

### SQL Injection Prevention

The API uses parameterized queries and input validation to prevent SQL injection attacks. All user inputs are sanitized and validated before being used in database operations.

### JSON Extraction and Patching

The API supports JSON path expressions for selective field updates and queries:
- `json_extract`: Used for querying specific fields in JSON data
- `json_set`: Used for updating specific fields in JSON data
- `json_patch`: Used for applying multiple updates to JSON data

### Category Restrictions

Category names must match the regex pattern `^[a-zA-Z0-9-_]{2,10}$`:
- Length between 2-10 characters
- Only alphanumeric characters, hyphens, and underscores allowed

### Auto-Updating Timestamps

The `updated` timestamp is automatically refreshed whenever a record is modified, providing an audit trail of changes.