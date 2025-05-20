# 通用数据 API v4 文档

## 目录

- [概述](#概述)
- [基础URL](#基础url)
- [数据模型](#数据模型)
- [认证](#认证)
- [接口端点](#接口端点)
  - [创建数据条目](#创建数据条目)
  - [获取数据条目](#获取数据条目)
  - [查询数据条目](#查询数据条目)
  - [统计数据条目](#统计数据条目)
  - [更新数据条目](#更新数据条目)
  - [删除数据条目](#删除数据条目)
- [技术说明](#技术说明)
  - [类别验证](#类别验证)
  - [JSON字段提取](#json字段提取)
  - [数据类型处理](#数据类型处理)
  - [字段选择和扁平化](#字段选择和扁平化)
- [错误处理](#错误处理)

## 概述

此API提供了一套全面的接口端点，用于管理具有JSON功能的通用数据存储。API v4遵循RESTful原则，支持对用户定义类别的数据进行CRUD操作（创建、读取、更新、删除）。

该服务将数据存储为JSON对象，包含系统字段（id、类别、时间戳）和用户定义的字段。API支持通过JSON路径提取进行复杂查询，提供灵活的数据管理解决方案。

## 基础URL

所有端点都相对于: `/api/v4/data/`

## 数据模型

每个数据条目包含:

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| id | 整数 | 唯一标识符，自动生成 |
| cat | 字符串 | 类别名称 |
| data | JSON对象 | 存储为JSON的用户定义数据 |
| is_deleted | 布尔值 | 软删除标志 |
| created | 时间戳 | 创建时间戳（毫秒） |
| updated | 时间戳 | 最后更新时间戳（毫秒） |

## 认证

*提供的代码中未指定认证要求。*

## 接口端点

### 创建数据条目

```
POST /api/v4/data/:category/insert
```

在指定类别中创建新的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称（2-20个字符，字母数字、破折号、下划线） |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| unique | 布尔值 | 可选。如果为true，确保该类别只存在一条记录。如果记录已存在，将会更新而不是插入。默认为false |

#### 请求体

包含要存储的数据的JSON对象。系统字段名称（`id`、`cat`、`data`、`is_deleted`、`created`、`updated`）不能用作键。

#### 示例

请求:
```json
POST /api/v4/data/products/insert
{
  "name": "产品1",
  "price": 99.99,
  "active": true,
  "tags": ["新品", "精选"]
}
```

响应:
```json
{
  "id": 42,
  "cat": "products",
  "name": "产品1",
  "price": 99.99,
  "active": true,
  "tags": ["新品", "精选"],
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

使用unique参数:
```json
POST /api/v4/data/site_settings/insert?unique=true
{
  "theme": "dark",
  "maintenance_mode": false
}
```

### 获取数据条目

```
GET /api/v4/data/:category/get
```

通过ID从指定类别中获取单个数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| id | 整数 | 可选。要获取的条目ID。如果未提供，API会查找该类别下的唯一一条记录 |
| select | 字符串 | 可选。要返回的字段的逗号分隔列表。默认为"*"（所有字段） |
| slim | 布尔值 | 可选。如果为true，则仅返回数据对象，不包含系统字段。默认为false |

#### 示例

请求:
```
GET /api/v4/data/products/get?id=42&select=name,price
```

响应:
```json
{
  "id": 42,
  "cat": "products",
  "name": "产品1",
  "price": 99.99,
  "created": 1715471025000,
  "updated": 1715471025000,
  "is_deleted": false
}
```

对于单记录类别:
```
GET /api/v4/data/site_settings/get
```

### 查询数据条目

```
GET /api/v4/data/:category/query
```

基于查询参数从指定类别中检索多个数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| select | 字符串 | 可选。要返回的字段的逗号分隔列表。默认为"*"（所有字段） |
| limit | 字符串/数字 | 可选。格式："偏移量,数量"或仅数量。默认为"0,10" |
| where | 字符串 | 可选。用于过滤的类SQL条件。JSON字段会自动使用正确的语法提取 |
| order_by | 字符串 | 可选。用于排序的字段。默认为"id desc" |
| slim | 布尔值 | 可选。如果为true，则仅返回数据对象，不包含系统字段。默认为false |
| include_deleted | 布尔值 | 可选。如果为true，包括软删除的条目。默认为false |

#### Where子句语法

where子句支持标准SQL比较运算符（`=`、`!=`、`>`、`<`、`>=`、`<=`、`like`）和`AND`/`OR`运算符组合条件。

对于嵌套的JSON属性，只需使用属性名。API将自动将其转换为适当的JSON提取语法。

示例:
- `price>50`
- `status="active" AND price<100`
- `name like "%product%" OR tags="featured"`

#### 示例

请求:
```
GET /api/v4/data/products/query?where=price>50 AND active=true&order_by=price asc&limit=0,5
```

响应:
```json
[
  {
    "id": 42,
    "cat": "products",
    "name": "产品1",
    "price": 99.99,
    "active": true,
    "tags": ["新品", "精选"],
    "created": 1715471025000,
    "updated": 1715471025000,
    "is_deleted": false
  },
  {
    "id": 43,
    "cat": "products",
    "name": "产品2",
    "price": 149.99,
    "active": true,
    "tags": ["精选"],
    "created": 1715471095000,
    "updated": 1715471095000,
    "is_deleted": false
  }
]
```

### 统计数据条目

```
GET /api/v4/data/:category/count
```

统计指定类别中符合给定条件的数据条目数量。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| where | 字符串 | 可选。用于过滤的类SQL条件 |
| include_deleted | 布尔值 | 可选。如果为true，包括软删除的条目。默认为false |

#### 示例

请求:
```
GET /api/v4/data/products/count?where=price>50 AND active=true
```

响应:
```json
{
  "rows": 42
}
```

### 更新数据条目

```
POST /api/v4/data/:category/update
```

更新指定类别中的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| id | 整数 | 要更新的条目的ID |
| override_data | 布尔值 | 可选。如果为true，替换整个数据对象。如果为false，执行JSON补丁操作。默认为false |

#### 请求体

包含要更新的字段的JSON对象。

#### 示例

部分更新（JSON补丁）:
```json
POST /api/v4/data/products/update?id=42
{
  "price": 129.99,
  "tags": ["新品", "精选", "促销"]
}
```

完全数据覆盖:
```json
POST /api/v4/data/products/update?id=42&override_data=true
{
  "name": "产品1 - 已更新",
  "price": 129.99,
  "active": true,
  "tags": ["新品", "精选", "促销"]
}
```

响应:
```json
{
  "affected_rows": 1
}
```

### 删除数据条目

```
POST /api/v4/data/:category/delete
```

删除指定类别中的数据条目。

#### 路径参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| category | 字符串 | 类别名称 |

#### 查询参数

| 参数 | 类型 | 描述 |
|-----------|------|-------------|
| id | 整数 | 可选。要删除的条目的ID。如果delete_all为false，则必需 |
| delete_all | 布尔值 | 可选。如果为true，删除该类别中的所有条目。默认为false |
| hard_delete | 布尔值 | 可选。如果为true，永久删除条目。如果为false，执行软删除。默认为false |

#### 示例

单个条目删除:
```
POST /api/v4/data/products/delete?id=42&hard_delete=false
```

删除所有数据:
```
POST /api/v4/data/products/delete?delete_all=true&hard_delete=true
```

响应:
```json
{
  "affected_rows": 1
}
```

## 技术说明

### 类别验证

类别必须:
- 长度为2-20个字符
- 仅包含字母数字字符、破折号和下划线
- 匹配正则表达式模式: `^[a-zA-Z0-9-_]{2,20}$`

### JSON字段提取

API自动处理查询的JSON字段提取。在对`data`列中的JSON字段进行查询或过滤时，API会将标准字段引用转换为适当的JSON提取语法。

例如，像这样的查询:
```
where=price>50 AND name="产品1"
```

会自动转换为:
```sql
WHERE json_extract(data, '$.price') > 50 AND json_extract(data, '$.name') = "产品1"
```

v4 API改进了对复杂条件的支持，包括在where子句中正确处理AND/OR运算符。

### 数据类型处理

API自动处理查询参数的数据类型转换:
- 布尔值: "true"和"false"（不区分大小写）会转换为布尔值
- 空值: "null"或空字符串会转换为null
- 数字: 数字字符串会转换为整数或浮点数
- 字符串: 所有其他值都被视为字符串

### 字段选择和扁平化

API支持:
- 使用`select`参数选择特定字段
- 通过将系统字段和JSON数据字段合并为单个对象来"扁平化"结果（当`slim=false`时）
- 仅返回数据对象（当`slim=true`时）

## 错误处理

API会针对不同的错误场景返回适当的HTTP状态码:
- 400 Bad Request: 无效的参数或请求体
- 404 Not Found: 未找到资源
- 500 Internal Server Error: 服务器端处理错误

错误响应包含一条描述性消息，以帮助调试。