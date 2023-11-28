import re

def test_finditer():
    # r =
    for r in re.finditer("(\w)haha(\w)","aahahabbaahahabb"):
        print(r.start())
        print(r.end())
        print(r.groups())
        # print(r)


def test_search():
    r = re.search(r"""(?m)([urbURB]?(?:''(?!')|""(?!")|'{6}|"{6}|'(?:[^\\']|\\.)+?'|"(?:[^\\"]|\\.)+?"|'{3}(?:[^\\]|\\.|\n)+?'{3}|"{3}(?:[^\\]|\\.|\n)+?"{3}))|(#.*)|([\[\{\(])|([\]\}\)])|^([ \t]*(?:if|for|while|with|try|def|class)\b)|^([ \t]*(?:elif|else|except|finally)\b)|((?:^|;)[ \t]*end[ \t]*(?=(?:%>[ \t]*)?\r?$|;|#))|(%>[ \t]*(?=\r?$))|(\r?\n)""","""<ul>
    % for article in articles:
    <li>
        <a href="/article/{{article.id}}">{{article.title}}</a>
    </li>
    %end

    my name is :{{name}}
</ul>
    """)

    print(r.start())
    print(r.end())
    print(r.group(1))
    # print(r)

test_search()
# test_finditer()
