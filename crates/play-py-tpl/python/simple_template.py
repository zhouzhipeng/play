import re


def html_escape(string):
    ''' Escape HTML special characters ``&<>`` and quotes ``'"``. '''
    return string.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;') \
        .replace('"', '&quot;').replace("'", '&#039;')


def touni(s, enc='utf8', err='strict'):
    return s.decode(enc, err) if isinstance(s, bytes) else s


class StplParser(object):
    ''' Parser for stpl templates. '''
    _re_cache = {}  #: Cache for compiled re patterns
    # This huge pile of voodoo magic splits python code into 8 different tokens.
    # 1: All kinds of python strings (trust me, it works)
    _re_tok = '([urbURB]?(?:\'\'(?!\')|""(?!")|\'{6}|"{6}' \
              '|\'(?:[^\\\\\']|\\\\.)+?\'|"(?:[^\\\\"]|\\\\.)+?"' \
              '|\'{3}(?:[^\\\\]|\\\\.|\\n)+?\'{3}' \
              '|"{3}(?:[^\\\\]|\\\\.|\\n)+?"{3}))'
    _re_inl = _re_tok.replace('|\\n', '')  # We re-use this string pattern later
    # 2: Comments (until end of line, but not the newline itself)
    _re_tok += '|(#.*)'
    # 3,4: Open and close grouping tokens
    _re_tok += '|([\\[\\{\\(])'
    _re_tok += '|([\\]\\}\\)])'
    # 5,6: Keywords that start or continue a python block (only start of line)
    _re_tok += '|^([ \\t]*(?:if|for|while|with|try|def|class)\\b)' \
               '|^([ \\t]*(?:elif|else|except|finally)\\b)'
    # 7: Our special 'end' keyword (but only if it stands alone)
    _re_tok += '|((?:^|;)[ \\t]*end[ \\t]*(?=(?:%(block_close)s[ \\t]*)?\\r?$|;|#))'
    # 8: A customizable end-of-code-block template token (only end of line)
    _re_tok += '|(%(block_close)s[ \\t]*(?=\\r?$))'
    # 9: And finally, a single newline. The 10th token is 'everything else'
    _re_tok += '|(\\r?\\n)'

    # Match the start tokens of code areas in a template
    _re_split = '(?m)^[ \t]*(\\\\?)((%(line_start)s)|(%(block_start)s))(%%?)'
    # Match inline statements (may contain python strings)
    _re_inl = '(?m)%%(inline_start)s((?:%s|[^\'"\n]*?)+)%%(inline_end)s' % _re_inl
    _re_tok = '(?m)' + _re_tok

    default_syntax = '<% %> % {{ }}'

    def __init__(self, source, syntax=None, encoding='utf8'):
        self.source, self.encoding = touni(source, encoding), encoding
        self.set_syntax(syntax or self.default_syntax)
        self.code_buffer, self.text_buffer = [], []
        self.lineno, self.offset = 1, 0
        self.indent, self.indent_mod = 0, 0
        self.paren_depth = 0

    def get_syntax(self):
        ''' Tokens as a space separated string (default: <% %> % {{ }}) '''
        return self._syntax

    def set_syntax(self, syntax):
        self._syntax = syntax
        self._tokens = syntax.split()
        if not syntax in self._re_cache:
            names = 'block_start block_close line_start inline_start inline_end'
            etokens = map(re.escape, self._tokens)
            pattern_vars = dict(zip(names.split(), etokens))
            patterns = (self._re_split, self._re_tok, self._re_inl)
            patterns = [re.compile(p % pattern_vars) for p in patterns]
            self._re_cache[syntax] = patterns
        self.re_split, self.re_tok, self.re_inl = self._re_cache[syntax]

    syntax = property(get_syntax, set_syntax)

    def translate(self):
        if self.offset: raise RuntimeError('Parser is a one time instance.')
        while True:
            m = self.re_split.search(self.source[self.offset:])
            if m:
                text = self.source[self.offset:self.offset + m.start()]
                self.text_buffer.append(text)
                self.offset += m.end()
                if m.group(1):  # New escape syntax
                    line, sep, _ = self.source[self.offset:].partition('\n')
                    self.text_buffer.append(m.group(2) + m.group(5) + line + sep)
                    self.offset += len(line + sep) + 1
                    continue
                elif m.group(5):  # Old escape syntax

                    line, sep, _ = self.source[self.offset:].partition('\n')
                    self.text_buffer.append(m.group(2) + line + sep)
                    self.offset += len(line + sep) + 1
                    continue
                self.flush_text()
                self.read_code(multiline=bool(m.group(4)))
            else:
                break
        self.text_buffer.append(self.source[self.offset:])
        self.flush_text()
        return ''.join(self.code_buffer)

    def read_code(self, multiline):
        code_line, comment = '', ''
        while True:
            m = self.re_tok.search(self.source[self.offset:])
            if not m:
                code_line += self.source[self.offset:]
                self.offset = len(self.source)
                self.write_code(code_line.strip(), comment)
                return
            code_line += self.source[self.offset:self.offset + m.start()]
            self.offset += m.end()
            _str, _com, _po, _pc, _blk1, _blk2, _end, _cend, _nl = m.groups()
            if (code_line or self.paren_depth > 0) and (_blk1 or _blk2):  # a if b else c
                code_line += _blk1 or _blk2
                continue
            if _str:  # Python string
                code_line += _str
            elif _com:  # Python comment (up to EOL)
                comment = _com
                if multiline and _com.strip().endswith(self._tokens[1]):
                    multiline = False  # Allow end-of-block in comments
            elif _po:  # open parenthesis
                self.paren_depth += 1
                code_line += _po
            elif _pc:  # close parenthesis
                if self.paren_depth > 0:
                    # we could check for matching parentheses here, but it's
                    # easier to leave that to python - just check counts
                    self.paren_depth -= 1
                code_line += _pc
            elif _blk1:  # Start-block keyword (if/for/while/def/try/...)
                code_line, self.indent_mod = _blk1, -1
                self.indent += 1
            elif _blk2:  # Continue-block keyword (else/elif/except/...)
                code_line, self.indent_mod = _blk2, -1
            elif _end:  # The non-standard 'end'-keyword (ends a block)
                self.indent -= 1
            elif _cend:  # The end-code-block template token (usually '%>')
                if multiline:
                    multiline = False
                else:
                    code_line += _cend
            else:  # \n
                self.write_code(code_line.strip(), comment)
                self.lineno += 1
                code_line, comment, self.indent_mod = '', '', 0
                if not multiline:
                    break

    def flush_text(self):
        text = ''.join(self.text_buffer)
        del self.text_buffer[:]
        if not text: return
        parts, pos, nl = [], 0, '\\\n' + '  ' * self.indent
        for m in self.re_inl.finditer(text):
            prefix, pos = text[pos:m.start()], m.end()
            if prefix:
                parts.append(nl.join(map(repr, prefix.splitlines(True))))
            if prefix.endswith('\n'): parts[-1] += nl
            parts.append(self.process_inline(m.group(1).strip()))
        if pos < len(text):
            prefix = text[pos:]
            lines = prefix.splitlines(True)
            if lines[-1].endswith('\\\\\n'):
                lines[-1] = lines[-1][:-3]
            elif lines[-1].endswith('\\\\\r\n'):
                lines[-1] = lines[-1][:-4]
            parts.append(nl.join(map(repr, lines)))
        code = '_printlist((%s,))' % ', '.join(parts)
        self.lineno += code.count('\n') + 1
        self.write_code(code)

    def process_inline(self, chunk):
        if chunk[0] == '!': return '_str(%s)' % chunk[1:]
        return '_escape(%s)' % chunk

    def write_code(self, line, comment=''):
        line, comment = self.fix_backward_compatibility(line, comment)
        code = '  ' * (self.indent + self.indent_mod)
        code += line.lstrip() + comment + '\n'
        self.code_buffer.append(code)

    def fix_backward_compatibility(self, line, comment):
        return line, comment


class TemplateError(Exception):
    def __init__(self, message):
        Exception(self, message)


class BaseTemplate(object):
    """ Base class and minimal API for template adapters """
    extensions = ['tpl', 'html', 'thtml', 'stpl']
    settings = {}  # used in prepare()
    defaults = {}  # used in render()

    def __init__(self, source=None, name=None, lookup=[], encoding='utf8', **settings):
        """ Create a new template.
        If the source parameter (str or buffer) is missing, the name argument
        is used to guess a template filename. Subclasses can assume that
        self.source and/or self.filename are set. Both are strings.
        The lookup, encoding and settings parameters are stored as instance
        variables.
        The lookup parameter stores a list containing directory paths.
        The encoding parameter should be used to decode byte strings or files.
        The settings parameter contains a dict for engine-specific settings.
        """
        self.name = name
        self.source = source.read() if hasattr(source, 'read') else source
        self.filename = source.filename if hasattr(source, 'filename') else None
        self.lookup = []
        self.encoding = encoding
        self.settings = self.settings.copy()  # Copy from class variable
        self.settings.update(settings)  # Apply

        # if not self.source and not self.filename:
        #     raise TemplateError('No template specified.')
        self.prepare(**self.settings)

    @classmethod
    def global_config(cls, key, *args):
        ''' This reads or sets the global settings stored in class.settings. '''
        if args:
            cls.settings = cls.settings.copy()  # Make settings local to class
            cls.settings[key] = args[0]
        else:
            return cls.settings[key]

    def prepare(self, **options):
        """ Run preparations (parsing, caching, ...).
        It should be possible to call this again to refresh a template or to
        update settings.
        """
        raise NotImplementedError

    def render(self, *args, **kwargs):
        """ Render the template with the specified local variables and return
        a single byte or unicode string. If it is a byte string, the encoding
        must match self.encoding. This method must be thread-safe!
        Local variables may be provided in dictionaries (args)
        or directly, as keywords (kwargs).
        """
        raise NotImplementedError


class AttributeDict(dict):
    def __init__(self, data):
        super(AttributeDict, self).__init__(data)
        self.__dict__ = self
        for name, value in data.items():
            setattr(self, name, self._wrap(value))

    def _wrap(self, value):
        if isinstance(value, (tuple, list, set, frozenset)):
            return type(value)([self._wrap(v) for v in value])
        else:
            return AttributeDict(value) if isinstance(value, dict) else value


class SimpleTemplate(BaseTemplate):

    def prepare(self, escape_func=html_escape, noescape=False, syntax=None, **ka):
        self.cache = {}
        enc = self.encoding
        self._str = lambda x: touni(x, enc)
        self._escape = lambda x: escape_func(touni(x, enc))
        self.syntax = syntax
        if noescape:
            self._str, self._escape = self._escape, self._str

    def co(self):

        if self.filename in global_cache:
            return global_cache[self.filename]
        else:
            # for test
            # raise Exception(self.code())
            c = compile(self.code(), self.filename or '<string>', 'exec')
            global_cache[self.filename] = c
            return c

    def code(self):
        source = self.source
        try:
            source, encoding = touni(source), 'utf8'
        except UnicodeError:

            source, encoding = touni(source, 'latin1'), 'latin1'
        parser = StplParser(source, encoding=encoding, syntax=self.syntax)
        code = parser.translate()
        self.encoding = parser.encoding
        return code

    def execute(self, _stdout, kwargs):
        env = self.defaults.copy()
        env.update(kwargs)
        env.update({'_stdout': _stdout, '_printlist': _stdout.extend, 'include': include,
                    'http_get': http_get,
                    'local_http_get': local_http_get,
                    'AttributeDict': AttributeDict,

                    '_str': self._str, '_escape': self._escape, 'get': env.get,
                    'setdefault': env.setdefault, 'defined': env.__contains__
                    })

        eval(self.co(), AttributeDict(env))

        return env

    def render(self, *args, **kwargs):
        """ Render the template using keyword arguments as local variables. """
        env = {};
        stdout = []
        for dictarg in args: env.update(dictarg)
        env.update(kwargs)
        self.execute(stdout, env)
        return ''.join(str(x) for x in stdout)


# global cache
global_cache = {}


def clear_cache(filename: str) -> bool:
    if filename in global_cache:
        del global_cache[filename]
        return True
    else:
        return False


def render_tpl(source: str, filename: str, args: dict) -> str:
    t = SimpleTemplate(source, noescape=True)
    t.filename = filename
    s = t.render(**args)
    return s


import json


def render_tpl_with_str_args(source: str, filename: str, str_args: str, use_cache: bool) -> str:
    if not use_cache:
        clear_cache(filename)
    r = render_tpl(source, filename, json.loads(str_args))
    if not use_cache:
        clear_cache(filename)
    return r


cached_or_not = {}


def cache_template(filename: str, source: str):
    t = SimpleTemplate(source, noescape=True)
    t.filename = filename
    t.co()
    cached_or_not[filename] = True
    print("template :" + filename + "  cached.")


debug_mode = False


def set_debug_mode(mode: bool):
    global debug_mode
    debug_mode = mode
    # print("simple_template >> set debug mode = "+ str(mode))


def include(file_name: str, **kwargs) -> str:
    import foo  # rust module.
    if debug_mode:
        clear_cache(file_name)
        content = foo.read_file(file_name)
    else:
        if file_name in global_cache:
            content = global_cache[file_name]
        else:
            content = foo.read_file(file_name)

    return render_tpl(content, file_name, kwargs)


def http_get(url: str) -> str:
    import foo  # rust module.
    return foo.http_get(url)
def local_http_get(uri: str) -> str:
    import foo  # rust module.
    return foo.local_http_get(uri)


if __name__ == '__main__':
    # test_data = {"ss":"bb", "aa":{"name":"111"}}
    # test_data = AttributeDict(test_data)
    # print(test_data.aa.name)

    args = {"ss": "bb", "aa": {"name": "111"}}
    # local_map['__ret__'] =render_tpl()
    # todo: below code will block python interpreter
    print(render_tpl("{{ss}} {{symbol_change_asdfsdfs12sdfs}", "<tmp>", args))
