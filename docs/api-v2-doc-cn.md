# 通用数据 API 文档

本文档概述了一个灵活的数据存储系统的REST API。该API提供了一种通用方式来存储、检索、更新和删除按类别组织的数据。

## 目录

1. [API概述](#api概述)
2. [端点](#端点)
3. [操作](#操作)
4. [参数](#参数)
5. [示例](#示例)
6. [错误处理](#错误处理)

## API概述

通用数据API使用基于类别的结构来组织不同类型的数据。每个数据条目包括：

- 类别(`cat`)，用于分组相似数据
- JSON数据负载
- 用于跟踪的系统字段(ID、创建时间、更新时间、删除状态)

可以通过标准化操作(插入、查询、更新、删除)在所有类别中操作数据。

## 端点

API提供三个主要端点：

| 方法   | 端点                                  | 描述                                   |
|--------|---------------------------------------|--------------------------------------|
| GET    | `/api/v2/data/:category/:action`      | 使用查询字符串参数执行操作              |
| POST   | `/api/v2/data/:category/:action`      | 使用JSON请求体执行操作                 |
| GET    | `/api/v2/data/:category/:action/:hex` | 使用十六进制编码的参数字符串执行操作     |

**路径参数：**
- `:category` - 数据类别标识符(必须匹配模式 `^[a-zA-Z0-9-_]{2,10}$`)
- `:action` - 以下之一：`insert`、`update`、`query`或`delete`
- `:hex`(可选) - 操作的十六进制编码JSON参数

## 操作

### 1. 插入(Insert)

向指定类别添加新记录。

**参数：**
- 任何包含键值对的有效JSON对象
- 系统字段(`id`、`cat`、`data`、`is_deleted`、`created`、`updated`)不能用作键

**示例：**
```
POST /api/v2/data/users/insert
{
  "name": "John Doe",
  "email": "john@example.com",
  "active": true
}
```

**响应：**
返回包括系统字段在内的插入数据，以扁平化格式。

### 2. 查询(Query)

从指定类别检索数据。

**参数：**
- `id`(可选)：通过ID检索特定记录
- `select`(可选)：要返回的字段的逗号分隔列表(默认：`*`表示所有字段)
- `limit`(可选)：分页控制，格式为`offset,count`(默认：`0,10`)
- `where`(可选)：SQL类似的WHERE条件(支持AND运算符)
- `order_by`(可选)：SQL类似的ORDER BY子句(默认：`id desc`)
- `slim`(可选)：当为true时，仅返回数据内容而不包含系统字段(默认：`false`)
- `count`(可选)：当为true时，仅返回匹配记录的计数(默认：`false`)
- `include_deleted`(可选)：当为true时，包括软删除的记录(默认：`false`)

**示例：**
```
GET /api/v2/data/users/query?id=123
GET /api/v2/data/users/query?select=name,email&limit=0,20&where=active=true&order_by=name asc
GET /api/v2/data/users/query?count=true&where=active=true
```

**响应：**
- 对于单条记录查询：返回作为JSON对象的记录
- 对于列表查询：返回作为JSON对象数组的记录
- 对于计数查询：返回计数作为纯数字

### 3. 更新(Update)

更新指定类别中的现有记录。

**参数：**
- `id`：要更新的记录的ID
- `set`：字段赋值的逗号分隔列表，格式为`field1=value1,field2=value2`

**示例：**
```
GET /api/v2/data/users/update?id=123&set=name=Jane Doe,active=false
POST /api/v2/data/users/update
{
  "id": 123,
  "set": "name=Jane Doe,active=false"
}
```

**响应：**
返回受影响的行数(通常为1)。

### 4. 删除(Delete)

从指定类别中删除记录。

**参数：**
- `id`(可选)：要删除的记录的ID
- `delete_all`(可选)：当为true时，删除类别中的所有记录(默认：`false`)
- `hard_delete`(可选)：当为true时，永久删除记录；否则执行软删除(默认：`false`)

**示例：**
```
GET /api/v2/data/users/delete?id=123
GET /api/v2/data/users/delete?delete_all=true&hard_delete=true
```

**响应：**
返回受影响的行数。

## 参数

### 类别参数

`:category`路径参数必须遵循以下规则：
- 必须是2-10个字符长
- 只能包含字母、数字、连字符和下划线
- 用于将相关数据分组在一起

### 查询参数

#### Limit参数格式
`limit`参数接受两种格式：
- `offset,count`格式：`0,10`表示从记录0开始，返回10条记录
- 单个数字：`10`表示从偏移量0开始返回10条记录

#### Where条件格式
`where`参数使用类SQL语法：
- 基本条件：`field=value`
- 支持的运算符：`=`、`!=`、`>`、`<`、`>=`、`<=`
- 多个条件：`field1=value1 AND field2=value2`
- 对于JSON字段：`json_field=value`自动转换为`json_extract(data, '$.json_field')=value`

#### Set参数格式
`set`参数使用逗号分隔的字段赋值列表：
- 格式：`field1=value1,field2=value2`
- 字段名必须是有效的标识符(字母、数字、下划线，以字母/下划线开头)

## 示例

### 基本用法示例

**1. 插入新记录：**
```
POST /api/v2/data/tasks/insert
{
  "title": "完成文档",
  "due_date": "2025-05-15",
  "priority": 1,
  "completed": false
}
```

**2. 查询记录：**
```
# 获取特定记录
GET /api/v2/data/tasks/query?id=42

# 列出所有高优先级未完成任务
GET /api/v2/data/tasks/query?where=priority=1 AND completed=false&order_by=due_date asc

# 计算所有已完成任务
GET /api/v2/data/tasks/query?where=completed=true&count=true
```

**3. 更新记录：**
```
POST /api/v2/data/tasks/update
{
  "id": 42,
  "set": "completed=true,completion_date=2025-05-07"
}
```

**4. 删除记录：**
```
# 软删除(标记为已删除)
GET /api/v2/data/tasks/delete?id=42

# 硬删除(永久删除)
GET /api/v2/data/tasks/delete?id=42&hard_delete=true

# 删除类别中的所有记录
GET /api/v2/data/old_tasks/delete?delete_all=true&hard_delete=true
```

**5. 使用十六进制编码参数：**
```
# 等同于具有多个参数的复杂查询
GET /api/v2/data/tasks/query/7B22736C696D223A747275652C227768657265223A22636F6D706C657465643D66616C7365227D
```

## 错误处理

API为各种错误情况返回适当的HTTP状态码和错误消息：

- 400 Bad Request：无效的参数或请求格式
- 404 Not Found：资源未找到
- 500 Internal Server Error：服务器端错误

错误响应包括描述性消息，以帮助诊断问题：

```json
{
  "error": "invalid `category` path : inv@lid, not match with : ^[a-z0-9-]{2,10}$"
}
```

### 常见错误场景

1. 无效的类别名称(必须匹配模式 `^[a-zA-Z0-9-_]{2,10}$`)
2. 无效的操作(必须是以下之一：insert、update、query、delete)
3. 缺少必需参数(例如，更新/删除操作的id)
4. 无效的set参数格式
5. 在插入操作中使用系统字段名
6. 无效的limit参数格式