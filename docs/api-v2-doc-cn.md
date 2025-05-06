# 通用数据 API 文档

## 概述

该 API 提供了一个灵活的数据存储系统，数据按类别进行组织。它允许存储、检索、更新和删除 JSON 数据，并提供各种过滤和查询选项。

## 基础 URL

```
/api/v2/data
```

## 端点

### 数据操作

- `GET /api/v2/data/:category/:action` - 使用查询参数执行操作
- `POST /api/v2/data/:category/:action` - 使用 JSON 正文执行操作
- `GET /api/v2/data/:category/:action/:hex` - 使用十六进制编码参数执行操作

## 请求方式
`GET`和`POST`本质没有太大区别， get方式上的kv和post请求json体中的kv是一一对应的。几乎所有请求都可以用
get方式来完成。

## 数据类别 (category)
符合名称要求即可，无需事先创建。

## 操作类型 (action)

API 支持四种主要操作类型：

1. `insert` - 创建新数据条目
2. `update` - 更新现有数据条目
3. `query` - 检索数据条目
4. `delete` - 删除数据条目（软删除或硬删除）

## 参数

### 查询参数

```
{
  "id": Optional<u32>,          // 特定记录 ID
  "select": Optional<String>,   // 要选择的字段（逗号分隔）
  "limit": u32,                 // 默认值：10
  "where": Optional<String>,    // 过滤条件
  "order_by": Optional<String>, // 排序顺序
  "less": bool,                 // 默认值：false - 返回简化数据
  "count": bool                 // 默认值：false - 返回计数而非数据
}
```

### 更新参数

```
{
  "id": u32,        // 要更新的记录 ID
  "set": String     // 更新表达式（格式："field1=value1,field2=value2"）
}
```

### 删除参数

```
{
  "id": u32,             // 要删除的记录 ID
  "hard_delete": bool    // 默认值：false - 如果为 true，则永久删除记录
}
```

## 数据模型

API 操作基于 `GeneralData` 模型，结构如下：

```
{
  "id": u32,                  // 唯一标识符
  "cat": String,              // 类别名称
  "data": String,             // 存储为字符串的 JSON 数据
  "is_deleted": bool,         // 删除标志
  "created": NaiveDateTime,   // 创建时间戳
  "updated": NaiveDateTime    // 最后更新时间戳
}
```

## 详细用法

### 插入操作

插入新数据：

**方法：** `GET` 或 `POST`  
**URL：** `/api/v2/data/:category/insert`  
**参数：** 包含要插入数据的 JSON 对象

注意：
- 系统字段（`id`、`cat`、`is_deleted`、`created`、`updated`）不能包含在插入操作中
- 返回包含所有字段的插入数据，包括生成的 ID

### 更新操作

更新现有数据：

**方法：** `GET` 或 `POST`  
**URL：** `/api/v2/data/:category/update`  
**参数：**
```
{
  "id": 123,
  "set": "field1=value1,field2=value2"
}
```

注意：
- `set` 参数必须遵循 `field1=value1,field2=value2` 格式
- 返回受影响的行数

### 查询操作

查询数据：

**方法：** `GET` 或 `POST`  
**URL：** `/api/v2/data/:category/query`  
**参数：**
```
{
  "id": 123,              // 可选：查询特定 ID
  "select": "field1,field2",  // 可选：要选择的字段
  "limit": 20,            // 默认值：10
  "where": "field1='value1' AND field2>100",  // 可选：过滤条件
  "order_by": "id desc",  // 可选：默认 "id desc"
  "less": true,           // 可选：仅返回数据字段内容
  "count": false          // 可选：返回计数而非数据
}
```

注意：
- 如果提供了 `id`，则返回单个记录
- 如果未提供 `id`，则返回记录列表
- `where` 条件支持类 SQL 语法，带有 AND 运算符
- 当 `less` 为 true 时，仅返回 JSON 数据内容
- 当 `count` 为 true 时，返回匹配记录的计数

### 删除操作

删除数据：

**方法：** `GET` 或 `POST`  
**URL：** `/api/v2/data/:category/delete`  
**参数：**
```
{
  "id": 123,
  "hard_delete": false  // 可选：默认为 false
}
```

注意：
- 默认执行软删除（设置 is_deleted=1）
- 如果 `hard_delete` 为 true，则永久删除记录
- 返回受影响的行数

## 高级功能

### 十六进制编码请求

对于更复杂的查询或避免 URL 编码问题：

**URL：** `/api/v2/data/:category/:action/:hex`

其中 `:hex` 是参数的十六进制编码 JSON 字符串。

示例：
```
/api/v2/data/users/query/7B226964223A313233...
```

十六进制值解码为包含查询参数的 JSON 字符串。

### JSON 字段提取

API 提供两种处理返回数据的方法：

1. **to_flat_map()**：将系统字段和 JSON 数据合并为扁平结构
2. **extract_data()**：仅返回 JSON 数据部分

使用查询 API 时：
- 如果 `less` 为 `false`（默认），返回带有系统字段的完整数据
- 如果 `less` 为 `true`，仅返回 JSON 数据内容

### 字段选择

`select` 参数允许仅检索特定字段：

- `"*"` 返回所有字段（默认）
- `"field1,field2,field3"` 仅返回指定字段
- 使用 `json_extract` 从 JSON 数据中提取字段
- 以结构化格式返回所选字段

## 示例

### 插入示例

```
POST /api/v2/data/users/insert
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com",
  "age": 30
}
```

响应：
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

### 查询示例

```
GET /api/v2/data/users/query?id=1
```

响应：
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

### 列表查询示例

```
GET /api/v2/data/users/query?limit=10&where=age>25&order_by=name%20asc
```

响应：
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

### 更新示例

```
POST /api/v2/data/users/update
Content-Type: application/json

{
  "id": 1,
  "set": "name=Jane Doe,age=31"
}
```

响应：
```
1
```

### 删除示例

```
POST /api/v2/data/users/delete
Content-Type: application/json

{
  "id": 1,
  "hard_delete": false
}
```

响应：
```
1
```

## 错误处理

API 针对不同情况返回适当的错误消息：

- 无效的类别或操作
- 无效的参数
- 数据库错误
- 数据未找到
- 权限问题

错误遵循一致的格式，附有描述性消息，以帮助诊断问题。

## 特殊功能

### SQL 注入防护

API 使用参数化查询和输入验证来防止 SQL 注入攻击。所有用户输入在用于数据库操作之前都经过净化和验证。

### JSON 提取和修补

API 支持 JSON 路径表达式，用于选择性字段更新和查询：
- `json_extract`：用于查询 JSON 数据中的特定字段
- `json_set`：用于更新 JSON 数据中的特定字段
- `json_patch`：用于对 JSON 数据应用多个更新

### 类别限制

类别名称必须匹配正则表达式模式 `^[a-zA-Z0-9-_]{2,10}$`：
- 长度在 2-10 个字符之间
- 只允许字母数字字符、连字符和下划线

### 自动更新时间戳

每当记录被修改时，`updated` 时间戳会自动刷新，提供更改的审计跟踪。