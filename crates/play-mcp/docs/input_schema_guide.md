# Input Schema 格式指南

## 概述

`input_schema` 使用 [JSON Schema](https://json-schema.org/) 格式来定义工具的输入参数结构。这是 MCP (Model Context Protocol) 规范的一部分。

## 基本结构

```json
{
  "type": "object",
  "properties": {
    // 参数定义
  },
  "required": ["param1", "param2"],  // 必需参数列表
  "additionalProperties": false       // 是否允许额外属性
}
```

## 数据类型

### 1. 字符串 (string)
```json
{
  "type": "string",
  "description": "参数描述",
  "default": "默认值",
  "enum": ["option1", "option2"],  // 枚举值
  "minLength": 1,                  // 最小长度
  "maxLength": 100,                // 最大长度
  "pattern": "^[A-Z].*"            // 正则表达式
}
```

### 2. 数字 (number/integer)
```json
{
  "type": "number",      // 或 "integer" 表示整数
  "description": "数值参数",
  "default": 0,
  "minimum": 0,          // 最小值
  "maximum": 100,        // 最大值
  "exclusiveMinimum": 0, // 不包含最小值
  "exclusiveMaximum": 100 // 不包含最大值
}
```

### 3. 布尔值 (boolean)
```json
{
  "type": "boolean",
  "description": "开关参数",
  "default": false
}
```

### 4. 数组 (array)
```json
{
  "type": "array",
  "description": "数组参数",
  "items": {
    "type": "string"    // 数组元素类型
  },
  "minItems": 1,        // 最少元素数
  "maxItems": 10,       // 最多元素数
  "uniqueItems": true   // 元素是否唯一
}
```

### 5. 对象 (object)
```json
{
  "type": "object",
  "description": "复杂对象",
  "properties": {
    "field1": {
      "type": "string"
    },
    "field2": {
      "type": "number"
    }
  },
  "required": ["field1"]
}
```

## 实际示例

### 示例 1: HTTP 请求工具
```json
{
  "type": "object",
  "properties": {
    "url": {
      "type": "string",
      "description": "The URL to request",
      "format": "uri"  // URI 格式验证
    },
    "method": {
      "type": "string",
      "description": "HTTP method",
      "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
      "default": "GET"
    },
    "headers": {
      "type": "object",
      "description": "Optional HTTP headers",
      "additionalProperties": {
        "type": "string"
      }
    },
    "body": {
      "type": "string",
      "description": "Optional request body"
    },
    "timeout": {
      "type": "integer",
      "description": "Request timeout in seconds",
      "minimum": 1,
      "maximum": 300,
      "default": 30
    }
  },
  "required": ["url"]
}
```

### 示例 2: 文件操作工具
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "File path"
    },
    "content": {
      "type": "string",
      "description": "File content"
    },
    "encoding": {
      "type": "string",
      "description": "File encoding",
      "enum": ["utf-8", "ascii", "base64"],
      "default": "utf-8"
    },
    "create_dirs": {
      "type": "boolean",
      "description": "Create parent directories if they don't exist",
      "default": false
    }
  },
  "required": ["path", "content"]
}
```

### 示例 3: 数据查询工具
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query",
      "minLength": 1
    },
    "filters": {
      "type": "object",
      "properties": {
        "category": {
          "type": "string",
          "enum": ["docs", "code", "issues"]
        },
        "date_range": {
          "type": "object",
          "properties": {
            "from": {
              "type": "string",
              "format": "date"
            },
            "to": {
              "type": "string",
              "format": "date"
            }
          }
        }
      }
    },
    "limit": {
      "type": "integer",
      "description": "Maximum number of results",
      "minimum": 1,
      "maximum": 100,
      "default": 10
    },
    "sort": {
      "type": "string",
      "enum": ["relevance", "date", "popularity"],
      "default": "relevance"
    }
  },
  "required": ["query"]
}
```

## 高级特性

### 1. 条件模式 (Conditional Schemas)
```json
{
  "type": "object",
  "properties": {
    "type": {
      "type": "string",
      "enum": ["file", "url"]
    }
  },
  "allOf": [
    {
      "if": {
        "properties": { "type": { "const": "file" } }
      },
      "then": {
        "properties": {
          "path": { "type": "string" }
        },
        "required": ["path"]
      }
    },
    {
      "if": {
        "properties": { "type": { "const": "url" } }
      },
      "then": {
        "properties": {
          "url": { "type": "string", "format": "uri" }
        },
        "required": ["url"]
      }
    }
  ]
}
```

### 2. 组合模式 (Combining Schemas)
```json
{
  "type": "object",
  "properties": {
    "data": {
      "oneOf": [  // 必须匹配其中一个
        {
          "type": "string"
        },
        {
          "type": "array",
          "items": { "type": "string" }
        }
      ]
    }
  }
}
```

### 3. 引用定义 (Definitions)
```json
{
  "type": "object",
  "definitions": {
    "address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" },
        "city": { "type": "string" },
        "country": { "type": "string" }
      }
    }
  },
  "properties": {
    "billing_address": { "$ref": "#/definitions/address" },
    "shipping_address": { "$ref": "#/definitions/address" }
  }
}
```

## 格式验证器

JSON Schema 支持的格式验证器：

- `date-time`: ISO 8601 日期时间
- `date`: ISO 8601 日期
- `time`: ISO 8601 时间
- `email`: 电子邮件地址
- `hostname`: 主机名
- `ipv4`: IPv4 地址
- `ipv6`: IPv6 地址
- `uri`: URI
- `uri-reference`: URI 引用
- `uuid`: UUID
- `regex`: 正则表达式

## Rust 中的使用示例

```rust
use serde_json::json;

let input_schema = json!({
    "type": "object",
    "properties": {
        "message": {
            "type": "string",
            "description": "The message to process",
            "minLength": 1,
            "maxLength": 1000
        },
        "options": {
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["plain", "markdown", "html"],
                    "default": "plain"
                },
                "uppercase": {
                    "type": "boolean",
                    "default": false
                }
            }
        }
    },
    "required": ["message"]
});
```

## 最佳实践

1. **始终提供 description**: 为每个参数提供清晰的描述
2. **设置合理的默认值**: 对于可选参数，提供合理的默认值
3. **使用枚举限制选项**: 当参数只有有限选项时，使用 `enum`
4. **验证数据范围**: 使用 `minimum`、`maximum` 等限制数值范围
5. **标记必需参数**: 使用 `required` 数组明确标记必需参数
6. **使用格式验证**: 对特定格式的字符串使用 `format` 验证器

## 参考资源

- [JSON Schema 官方文档](https://json-schema.org/)
- [JSON Schema 验证器](https://www.jsonschemavalidator.net/)
- [MCP 规范](https://modelcontextprotocol.io/docs/concepts/tools)