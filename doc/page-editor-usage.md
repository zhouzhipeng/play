## how to use page editor

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