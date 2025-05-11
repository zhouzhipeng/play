# 通用数据 API 文档

## 概述

本 API 提供了一组全面的端点，用于管理具有 JSON 功能的通用数据存储。该 API 遵循 RESTful 原则，支持对用户定义类别的数据进行 CRUD 操作（创建、读取、更新、删除）。

服务将数据存储为 JSON 对象，包含系统字段（id、类别、时间戳）和用户自定义字段。API 支持复杂的查询操作和 JSON 路径提取，提供了灵活的数据管理解决方案。

## 基础 URL

所有端点的相对路径为：`/api/v3/data/`

## 数据模型

每个数据条目包含：

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| id | 整数 | 唯一标识符，自动生成 |
| cat | 字符串 | 类别名称 |
| data | JSON 对象 | 以 JSON 格式存储的用户自定义数据 |
| is_deleted | 布尔值 | 软删除标志 |
| created | 时间戳 | 创建时间戳（毫秒） |
| updated | 时间戳 | 最后更新时间戳（毫秒） |

## 身份验证

*提供的代码中未指定身份验证要求。*

## 端点

### 创建数据条目

```
POST /api/v3/data/:category/insert
```

在指定类别中创建新的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称（2-20个字符，字母数字、破折号、下划线） |

#### 请求体

包含要存储数据的 JSON 对象。系统字段名称（`id`、`cat`、`data`、`is_deleted`、`created`、`updated`）不能用作键。

#### 示例

请求：
```json
POST /api/v3/data/products/insert
{
  "name": "产品 1",
  "price": 99.99,
  "active": true,
  "tags": ["new", "featured"]
}
```

响应：
```json
{
  "id": 42,
  "cat": "products",
  "name": "产品 1",
  "price": 99.99,
  "active": true,
  "tags": ["new", "featured"],
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

### 检索数据条目

```
GET /api/v3/data/:category/get
```

通过 ID 从指定类别检索单个数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| id | 整数 | 要检索的条目 ID |
| select | 字符串 | 可选。以逗号分隔的返回字段列表。默认为 "*"（所有字段） |
| slim | 布尔值 | 可选。如果为 true，则仅返回数据对象而不包含系统字段。默认为 false |

#### 示例

请求：
```
GET /api/v3/data/products/get?id=42&select=name,price
```

响应：
```json
{
  "id": 42,
  "cat": "products",
  "name": "产品 1",
  "price": 99.99,
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

### 查询数据条目

```
GET /api/v3/data/:category/query
```

根据查询参数从指定类别检索多个数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| select | 字符串 | 可选。以逗号分隔的返回字段列表。默认为 "*"（所有字段） |
| limit | 字符串/数字 | 可选。格式："偏移量,数量" 或仅数量。默认为 "0,10" |
| where | 字符串 | 可选。用于过滤的类 SQL 条件。JSON 字段会自动以正确的语法提取 |
| order_by | 字符串 | 可选。排序字段。默认为 "id desc" |
| slim | 布尔值 | 可选。如果为 true，则仅返回数据对象而不包含系统字段。默认为 false |
| count | 布尔值 | 可选。如果为 true，同时返回总计数。默认为 false |
| include_deleted | 布尔值 | 可选。如果为 true，包含已软删除的条目。默认为 false |

#### Where 子句语法

where 子句支持标准 SQL 比较运算符（`=`、`!=`、`>`、`<`、`>=`、`<=`）和 `AND` 运算符来组合条件。

对于嵌套的 JSON 属性，只需使用属性名称。API 将自动将其转换为适当的 JSON 提取语法。

示例：
- `price>50`
- `status="active" AND price<100`
- `tags="featured" AND created>1614556800000`

#### 示例

请求：
```
GET /api/v3/data/products/query?where=price>50 AND active=true&order_by=price asc&limit=0,5
```

响应：
```json
[
  {
    "id": 42,
    "cat": "products",
    "name": "产品 1",
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
    "name": "产品 2",
    "price": 149.99,
    "active": true,
    "tags": ["featured"],
    "created": 1715471095000,
    "updated": 1715471095000,
    "is_deleted": false
  }
]
```

### 计数数据条目

```
GET /api/v3/data/:category/count
```

计算符合给定条件的指定类别中的数据条目数量。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| where | 字符串 | 可选。用于过滤的类 SQL 条件 |
| include_deleted | 布尔值 | 可选。如果为 true，包含已软删除的条目。默认为 false |

#### 示例

请求：
```
GET /api/v3/data/products/count?where=price>50 AND active=true
```

响应：
```
42
```

### 更新数据条目

```
POST /api/v3/data/:category/update
```

更新指定类别中的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 请求体

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| id | 整数 | 要更新的条目 ID |
| set | 对象 | 包含要更新的字段/值对的对象 |

#### 示例

请求：
```json
POST /api/v3/data/products/update
{
  "id": 42,
  "set": {
    "price": 129.99,
    "tags": ["new", "featured", "sale"]
  }
}
```

响应：
```json
{
  "affected_rows": 1
}
```

### 删除数据条目

```
POST /api/v3/data/:category/delete
```

删除指定类别中的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| id | 整数 | 可选。要删除的条目 ID。如果 delete_all 为 false，则必须提供 |
| delete_all | 布尔值 | 可选。如果为 true，删除类别中的所有条目。默认为 false |
| hard_delete | 布尔值 | 可选。如果为 true，永久删除条目。如果为 false，执行软删除。默认为 false |

#### 示例

请求：
```
POST /api/v3/data/products/delete?id=42&hard_delete=false
```

响应：
```json
{
  "affected_rows": 1
}
```

## 技术说明

### 类别验证

类别必须：
- 长度为 2-20 个字符
- 只包含字母数字字符、破折号和下划线
- 匹配正则表达式模式：`^[a-zA-Z0-9-_]{2,20}$`

### JSON 字段提取

API 自动处理 JSON 字段提取查询。当查询或过滤 `data` 列中的 JSON 字段时，API 将标准字段引用转换为适当的 JSON 提取语法。

例如，像这样的查询：
```
where=price>50 AND name="产品 1"
```

自动转换为：
```sql
WHERE json_extract(data, '$.price') > 50 AND json_extract(data, '$.name') = "产品 1"
```

### JSON 补丁

更新端点使用 JSON 补丁技术，只高效更新指定字段，不影响其他字段。

### 字段选择和扁平化

API 支持：
- 使用 `select` 参数选择特定字段
- 通过将系统字段和 JSON 数据字段组合到单个对象中来"扁平化"结果（当 `slim=false` 时）
- 仅返回数据对象（当 `slim=true` 时）

## 错误处理

API 为不同的错误场景返回适当的 HTTP 状态码：
- 400 Bad Request：无效的参数或请求体
- 404 Not Found：未找到资源
- 500 Internal Server Error：服务器端处理错误

错误响应包含描述性消息，以帮助调试。

## 特殊行为

### Lua 页面

当在"pages"类别中插入或更新数据，且标题以".lua"结尾时，API 会自动将内容保存到配置的 Lua 目录中的 .lua 文件中。