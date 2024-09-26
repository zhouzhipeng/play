## how to use page editor

### global variable
* params :   a map for storing request params on url
* envs :  a map for like 'host' info

by default, the page is static, if u want to use a dynamic page,
just remove the suffix '.html' in the uri.

and u can write python code (has some limit and minor changes, no indent need but 
'end' must be paired with 'def'/'if'/'for' etc. to mark a branch is completed.):

like below:

```txt
% a= int(params.get("a",0))
% b= int(params.get("b",0))


<%
def sub(a,b):
    return a-b
    end

%>

% arr = [1,2,3]

% for i in arr:
    i: {{i}}
% end

{
    "sum": {{a+b}},
    "sub": {{sub(a,b)}}
}


```
and u like visit like : /xxx?a=1&b=2


`params` is a python dictionary for request params in the query map of the url.
it will be injected by `pages_controller`



the grammar is :

```text
%  :  means a one line python code

<%  %> :  mean a python code block

{{ }}   ：  means to wrap a python variable or expression and display as output

```


### global functions
* http_get :   http get request
```python
resp = http_get("http://xxx")
```

* local_http_get :  visit current local http server 
```python
% import json
% email_list  = json.loads(local_http_get("/data/cat/mail_inbox?_json=true"))

```



### how to do CRUD in js code (using general data api)

```js
// insert data on a category
fetch(`/data/cat/${any_category}```, 
    {   method: "POST",headers: {'Content-Type': 'application/json'},
        body: JSON.stringify("{}")
    })
    .then(res=>res.json())
    .then(res=>note.id=res.id_or_count)

//actually , you can post any text, but if u use json , there is benefit from it.
```

```js
//delete data
fetch('/data/cat/$CAT/id/'+ note.id, {method: "DELETE"})
```

```js
//update data
let data = new URLSearchParams();

data.append("note",note.text);
data.append("timestamp", note.timestamp);
data.append("left",note.left);
data.append("top", note.top);
data.append("zindex", note.zIndex);
fetch(`/data/id-${note.id}`, {method: "PUT",headers: {'Content-Type': 'application/json'}, body: JSON.stringify(Object.fromEntries(data))})


```
```js
//query 
  fetch('/data/cat-StickyNotes?_json=true').then(res=>res.json())
    .then(result=>{
    
        for (var i = 0; i < result.length; ++i) {
            var row = result[i].data;
            var note = new Note();
            note.id = result[i]['id'];
            note.text = row['note'];
            note.timestamp = row['timestamp'];
            note.left = row['left'];
            note.top = row['top'];
            note.zIndex = row['zindex'];

           
            if (row['zindex'] > highestZ)
                highestZ = row['zindex'];
        }

      
    })

```


