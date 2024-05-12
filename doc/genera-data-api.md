# 通用数据api使用示例

> http状态码
* 200  处理成功，返回格式为json
* 500  处理异常，返回格式为text

## 1. 插入图书
> 插入接口分为两种
* 正常插入： -X POST 每次插入一条
* 全局一条： -X PUT  存在时覆盖

> category不需要预先配置或创建

### 1.1 插入一本普通图书信息到图书馆
```bash
HOST="http://127.0.0.1:3000"
# category可以为任意的合理的字符串
category="library"
# data 可以为任意字符串，不一定要传json字符串。 但是使用json字符串在后续查询或更新时有增益.
data='{"name":"book1", "author":"author1"}'

curl -X POST  --data  ${data}  "${HOST}/data/cat/${category}"
# {"id":52,"cat":"library","data":"{\"name\":\"book1\", \"author\":\"author1\"}","created":1715491280000,"updated":1715491280000}
```

### 1.2 插入一本“孤本”图书信息到图书馆（全球就这一本)
```bash
HOST="http://127.0.0.1:3000"
# category可以为任意的合理的字符串
category="super-library"
# data 可以为任意字符串，不一定要传json字符串。 但是使用json字符串在后续查询或更新时有增益.
data='{"name":"bestbook", "author":"bestauthor"}'

# 注意此时是 -X PUT , 当前category下必须没有或仅有一本书. 存在时，将覆盖老信息.
# 这只是一个“快捷方式”，针对需要全局数据的场景。 使用通用的添加和修改接口可以实现同样的效果。
curl -X PUT  --data  ${data}  "${HOST}/data/cat/${category}"
# {"id":52,"cat":"library","data":"{\"name\":\"book1\", \"author\":\"author1\"}","created":1715491280000,"updated":1715491280000}
# error resp >  Server Error: A global category should have only one item!
```


## 2. 修改图书信息(对孤本也适用,数据平等)
> 修改接口分为两种
* 覆盖内容：-X PUT  直接覆盖data内容
* 覆盖字段：-X PATCH 覆盖json data的某个字段，仅对插入时是json格式字符串有效，且需要是顶级字段.

> 修改接口并不需要category信息， 因为所有的数据id都是唯一的。

### 2.1 覆盖内容
```bash
HOST="http://127.0.0.1:3000"
# category可以为任意的合理的字符串
id=52
# data 可以为任意字符串，不一定要传json字符串。 但是使用json字符串在后续查询或更新时有增益.
data='{"name":"book2", "author":"author2"}'

curl -X PUT  --data  ${data}  "${HOST}/data/id/${id}"
# {"id":52,"cat":"library","data":"{\"name\":\"book2\", \"author\":\"author2\"}","created":1715491280000,"updated":1715493200000}
```

### 2.1 覆盖字段
```bash
HOST="http://127.0.0.1:3000"
# category可以为任意的合理的字符串
id=52
# data 可以为任意字符串，不一定要传json字符串。 但是使用json字符串在后续查询或更新时有增益.
query_param='name=book3'

curl -X PATCH "${HOST}/data/id/${id}?${query_param}"
# {"id":52,"cat":"library","data":"{\"name\":\"book3\",\"author\":\"author2\"}","created":1715491280000,"updated":1715493286000}
```

## 3. 查询图书
> 查询接口分为两种

```bash
HOST="http://127.0.0.1:3000"
# category可以为任意的合理的字符串
id=52
# data 可以为任意字符串，不一定要传json字符串。 但是使用json字符串在后续查询或更新时有增益.
query_param='name=book3'

curl -X PATCH "${HOST}/data/id/${id}?${query_param}"
# {"id":52,"cat":"library","data":"{\"name\":\"book3\",\"author\":\"author2\"}","created":1715491280000,"updated":1715493286000}
```