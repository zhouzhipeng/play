# General Data API Documentation

This document outlines the REST API for a flexible data storage system. The API provides a generic way to store, retrieve, update, and delete data organized by categories.

## Table of Contents

1. [API Overview](#api-overview)
2. [Endpoints](#endpoints)
3. [Actions](#actions)
4. [Parameters](#parameters)
5. [Examples](#examples)
6. [Error Handling](#error-handling)

## API Overview

The General Data API uses a category-based structure to organize different types of data. Each data entry consists of:

- A category (`cat`) which groups similar data
- A JSON data payload
- System fields for tracking (ID, creation time, update time, deletion status)

Data can be manipulated through standardized actions (insert, query, update, delete) across all categories.

## Endpoints

The API provides three main endpoints:

| Method | Endpoint                              | Description                                           |
|--------|---------------------------------------|-------------------------------------------------------|
| GET    | `/api/v2/data/:category/:action`      | Perform action using query string parameters          |
| POST   | `/api/v2/data/:category/:action`      | Perform action using JSON request body                |
| GET    | `/api/v2/data/:category/:action/:hex` | Perform action using hex-encoded parameter string     |

**Path Parameters:**
- `:category` - Data category identifier (must match pattern `^[a-zA-Z0-9-_]{2,10}$`)
- `:action` - One of: `insert`, `update`, `query`, or `delete`
- `:hex` (optional) - Hex-encoded JSON parameters for the action

## Actions

### 1. Insert

Adds a new record to the specified category.

**Parameters:**
- Any valid JSON object containing key-value pairs
- System fields (`id`, `cat`, `data`, `is_deleted`, `created`, `updated`) cannot be used as keys

**Example:**
```
POST /api/v2/data/users/insert
{
  "name": "John Doe",
  "email": "john@example.com",
  "active": true
}
```

**Response:**
Returns the inserted data including system fields, in a flattened format.

### 2. Query

Retrieves data from the specified category.

**Parameters:**
- `id` (optional): Retrieve a specific record by ID
- `select` (optional): Comma-separated list of fields to return (default: `*` for all fields)
- `limit` (optional): Pagination control in format `offset,count` (default: `0,10`)
- `where` (optional): SQL-like WHERE conditions (supports AND operator)
- `order_by` (optional): SQL-like ORDER BY clause (default: `id desc`)
- `slim` (optional): When true, returns only the data content without system fields (default: `false`)
- `count` (optional): When true, returns only the count of matching records (default: `false`)
- `include_deleted` (optional): When true, includes soft-deleted records (default: `false`)

**Examples:**
```
GET /api/v2/data/users/query?id=123
GET /api/v2/data/users/query?select=name,email&limit=0,20&where=active=true&order_by=name asc
GET /api/v2/data/users/query?count=true&where=active=true
```

**Response:**
- For single record queries: Returns the record as a JSON object
- For list queries: Returns an array of records as JSON objects
- For count queries: Returns the count as a plain number

### 3. Update

Updates an existing record in the specified category.

**Parameters:**
- `id`: ID of the record to update
- `set`: Comma-separated list of field assignments in format `field1=value1,field2=value2`

**Example:**
```
GET /api/v2/data/users/update?id=123&set=name=Jane Doe,active=false
POST /api/v2/data/users/update
{
  "id": 123,
  "set": "name=Jane Doe,active=false"
}
```

**Response:**
Returns the number of rows affected (usually 1).

### 4. Delete

Removes a record from the specified category.

**Parameters:**
- `id` (optional): ID of the record to delete
- `delete_all` (optional): When true, deletes all records in the category (default: `false`)
- `hard_delete` (optional): When true, permanently deletes the record(s); otherwise performs a soft delete (default: `false`)

**Examples:**
```
GET /api/v2/data/users/delete?id=123
GET /api/v2/data/users/delete?delete_all=true&hard_delete=true
```

**Response:**
Returns the number of rows affected.

## Parameters

### Category Parameter

The `:category` path parameter must follow these rules:
- Must be 2-10 characters long
- Can only contain letters, numbers, hyphens, and underscores
- Used to group related data together

### Query Parameters

#### Limit Parameter Format
The `limit` parameter accepts two formats:
- `offset,count` format: `0,10` means start from record 0 and return 10 records
- Single number: `10` means return 10 records starting from offset 0

#### Where Condition Format
The `where` parameter uses SQL-like syntax:
- Basic conditions: `field=value`
- Operators supported: `=`, `!=`, `>`, `<`, `>=`, `<=`
- Multiple conditions: `field1=value1 AND field2=value2`
- For JSON fields: `json_field=value` is automatically converted to `json_extract(data, '$.json_field')=value`

#### Set Parameter Format
The `set` parameter uses a comma-separated list of field assignments:
- Format: `field1=value1,field2=value2`
- Field names must be valid identifiers (letters, numbers, underscore, starting with letter/underscore)

## Examples

### Basic Usage Examples

**1. Insert a new record:**
```
POST /api/v2/data/tasks/insert
{
  "title": "Complete documentation",
  "due_date": "2025-05-15",
  "priority": 1,
  "completed": false
}
```

**2. Query records:**
```
# Get a specific record
GET /api/v2/data/tasks/query?id=42

# List all high-priority incomplete tasks
GET /api/v2/data/tasks/query?where=priority=1 AND completed=false&order_by=due_date asc

# Count all completed tasks
GET /api/v2/data/tasks/query?where=completed=true&count=true
```

**3. Update a record:**
```
POST /api/v2/data/tasks/update
{
  "id": 42,
  "set": "completed=true,completion_date=2025-05-07"
}
```

**4. Delete a record:**
```
# Soft delete (mark as deleted)
GET /api/v2/data/tasks/delete?id=42

# Hard delete (permanently remove)
GET /api/v2/data/tasks/delete?id=42&hard_delete=true

# Delete all records in a category
GET /api/v2/data/old_tasks/delete?delete_all=true&hard_delete=true
```

**5. Using hex-encoded parameters:**
```
# Equivalent to a complex query with many parameters
GET /api/v2/data/tasks/query/7B22736C696D223A747275652C227768657265223A22636F6D706C657465643D66616C7365227D
```

## Error Handling

The API returns appropriate HTTP status codes and error messages for various error conditions:

- 400 Bad Request: Invalid parameters or request format
- 404 Not Found: Resource not found
- 500 Internal Server Error: Server-side errors

Error responses include a descriptive message to help diagnose issues:

```json
{
  "error": "invalid `category` path : inv@lid, not match with : ^[a-z0-9-]{2,10}$"
}
```

### Common Error Scenarios

1. Invalid category name (must match pattern `^[a-zA-Z0-9-_]{2,10}$`)
2. Invalid action (must be one of: insert, update, query, delete)
3. Missing required parameters (e.g., id for update/delete)
4. Invalid set parameter format
5. Using system field names in insert operations
6. Invalid limit parameter format