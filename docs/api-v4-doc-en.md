# General Data API v4 Documentation

## Table of Contents

- [Overview](#overview)
- [Base URL](#base-url)
- [Data Model](#data-model)
- [Authentication](#authentication)
- [Endpoints](#endpoints)
  - [Create Data Entry](#create-data-entry)
  - [Retrieve Data Entry](#retrieve-data-entry)
  - [Query Data Entries](#query-data-entries)
  - [Count Data Entries](#count-data-entries)
  - [Update Data Entry](#update-data-entry)
  - [Delete Data Entry](#delete-data-entry)
- [Technical Notes](#technical-notes)
  - [Category Validation](#category-validation)
  - [JSON Field Extraction](#json-field-extraction)
  - [Data Types Handling](#data-types-handling)
  - [Field Selection and Flattening](#field-selection-and-flattening)
- [Error Handling](#error-handling)

## Overview

This API provides a comprehensive set of endpoints for managing general-purpose data storage with JSON capabilities. The API v4 follows RESTful principles and supports CRUD operations (Create, Read, Update, Delete) on data categorized by user-defined categories.

The service stores data as JSON objects with system fields (id, category, timestamps) and user-defined fields. The API supports complex querying with JSON path extraction, providing a flexible data management solution.

## Base URL

All endpoints are relative to: `/api/v4/data/`

## Data Model

Each data entry consists of:

| Field | Type | Description |
|-------|------|-------------|
| id | integer | Unique identifier, auto-generated |
| cat | string | Category name |
| data | JSON object | User-defined data stored as JSON |
| is_deleted | boolean | Soft deletion flag |
| created | timestamp | Creation timestamp (in milliseconds) |
| updated | timestamp | Last update timestamp (in milliseconds) |

## Authentication

*Authentication requirements are not specified in the provided code.*

## Endpoints

### Create Data Entry

```
POST /api/v4/data/:category/insert
```

Creates a new data entry in the specified category.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name (2-20 characters, alphanumeric, dash, underscore) |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| unique | boolean | Optional. If true, ensures only one record exists for this category. If a record already exists, it will be updated instead. Defaults to false |

#### Request Body

A JSON object containing the data to store. System field names (`id`, `cat`, `data`, `is_deleted`, `created`, `updated`) cannot be used as keys.

#### Example

Request:
```json
POST /api/v4/data/products/insert
{
  "name": "Product 1",
  "price": 99.99,
  "active": true,
  "tags": ["new", "featured"]
}
```

Response:
```json
{
  "id": 42,
  "cat": "products",
  "name": "Product 1",
  "price": 99.99,
  "active": true,
  "tags": ["new", "featured"],
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

With unique parameter:
```json
POST /api/v4/data/site_settings/insert?unique=true
{
  "theme": "dark",
  "maintenance_mode": false
}
```

### Retrieve Data Entry

```
GET /api/v4/data/:category/get
```

Retrieves a single data entry by ID from the specified category.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | integer | Optional. ID of the entry to retrieve. If not provided, the API expects exactly one record in the category |
| select | string | Optional. Comma-separated list of fields to return. Defaults to "*" (all fields) |
| slim | boolean | Optional. If true, returns only the data object without system fields. Defaults to false |

#### Example

Request:
```
GET /api/v4/data/products/get?id=42&select=name,price
```

Response:
```json
{
  "id": 42,
  "cat": "products",
  "name": "Product 1",
  "price": 99.99,
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

For single record categories:
```
GET /api/v4/data/site_settings/get
```

### Query Data Entries

```
GET /api/v4/data/:category/query
```

Retrieves multiple data entries from the specified category based on query parameters.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| select | string | Optional. Comma-separated list of fields to return. Defaults to "*" (all fields) |
| limit | string/number | Optional. Format: "offset,count" or just count. Defaults to "0,10" |
| where | string | Optional. SQL-like conditions for filtering. JSON fields are automatically extracted with proper syntax |
| order_by | string | Optional. Field(s) to sort by. Defaults to "id desc" |
| slim | boolean | Optional. If true, returns only the data objects without system fields. Defaults to false |
| include_deleted | boolean | Optional. If true, includes soft-deleted entries. Defaults to false |

#### Where Clause Syntax

The where clause supports standard SQL comparison operators (`=`, `!=`, `>`, `<`, `>=`, `<=`, `like`) and the `AND`/`OR` operators to combine conditions.

For nested JSON properties, simply use the property name. The API will automatically convert it to the appropriate JSON extraction syntax.

Examples:
- `price>50`
- `status="active" AND price<100`
- `name like "%product%" OR tags="featured"`

#### Example

Request:
```
GET /api/v4/data/products/query?where=price>50 AND active=true&order_by=price asc&limit=0,5
```

Response:
```json
[
  {
    "id": 42,
    "cat": "products",
    "name": "Product 1",
    "price": 99.99,
    "active": true,
    "tags": ["new", "featured"],
    "created": 1715471025000,
    "updated": 1715471025000,
    "is_deleted": false
  },
  {
    "id": 43,
    "cat": "products",
    "name": "Product 2",
    "price": 149.99,
    "active": true,
    "tags": ["featured"],
    "created": 1715471095000,
    "updated": 1715471095000,
    "is_deleted": false
  }
]
```

### Count Data Entries

```
GET /api/v4/data/:category/count
```

Counts data entries in the specified category that match the given criteria.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| where | string | Optional. SQL-like conditions for filtering |
| include_deleted | boolean | Optional. If true, includes soft-deleted entries. Defaults to false |

#### Example

Request:
```
GET /api/v4/data/products/count?where=price>50 AND active=true
```

Response:
```json
{
  "rows": 42
}
```

### Update Data Entry

```
POST /api/v4/data/:category/update
```

Updates a data entry in the specified category.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | integer | ID of the entry to update |
| override_data | boolean | Optional. If true, replaces the entire data object. If false, performs a JSON patch operation. Defaults to false |

#### Request Body

A JSON object containing the fields to update.

#### Example

Partial update (JSON patch):
```json
POST /api/v4/data/products/update?id=42
{
  "price": 129.99,
  "tags": ["new", "featured", "sale"]
}
```

Full data override:
```json
POST /api/v4/data/products/update?id=42&override_data=true
{
  "name": "Product 1 - Updated",
  "price": 129.99,
  "active": true,
  "tags": ["new", "featured", "sale"]
}
```

Response:
```json
{
  "affected_rows": 1
}
```

### Delete Data Entry

```
POST /api/v4/data/:category/delete
```

Deletes data entries in the specified category.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| category | string | Category name |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | integer | Optional. ID of the entry to delete. Required if delete_all is false |
| delete_all | boolean | Optional. If true, deletes all entries in the category. Defaults to false |
| hard_delete | boolean | Optional. If true, permanently deletes the entry(s). If false, performs a soft delete. Defaults to false |

#### Example

Single entry deletion:
```
POST /api/v4/data/products/delete?id=42&hard_delete=false
```

Delete all data:
```
POST /api/v4/data/products/delete?delete_all=true&hard_delete=true
```

Response:
```json
{
  "affected_rows": 1
}
```

## Technical Notes

### Category Validation

Categories must:
- Be 2-20 characters long
- Contain only alphanumeric characters, dashes, and underscores
- Match the regex pattern: `^[a-zA-Z0-9-_]{2,20}$`

### JSON Field Extraction

The API automatically handles JSON field extraction for queries. When querying or filtering on JSON fields in the `data` column, the API converts standard field references into the appropriate JSON extraction syntax.

For example, a query like:
```
where=price>50 AND name="Product 1"
```

Is automatically converted to:
```sql
WHERE json_extract(data, '$.price') > 50 AND json_extract(data, '$.name') = "Product 1"
```

The v4 API has improved support for complex conditions including proper handling of AND/OR operators in the where clause.

### Data Types Handling

The API automatically handles data type conversion for query parameters:
- Boolean values: "true" and "false" (case-insensitive) are converted to boolean values
- Null values: "null" or empty strings are converted to null
- Numbers: Numeric strings are converted to integers or floating-point numbers
- Strings: All other values are treated as strings

### Field Selection and Flattening

The API supports:
- Selecting specific fields using the `select` parameter
- "Flattening" results by combining system fields and JSON data fields into a single object (when `slim=false`)
- Returning only the data object (when `slim=true`)

## Error Handling

The API returns appropriate HTTP status codes for different error scenarios:
- 400 Bad Request: Invalid parameters or request body
- 404 Not Found: Resource not found
- 500 Internal Server Error: Server-side processing error

Error responses include a descriptive message to help with debugging.