from simple_template import render_tpl, global_cache
import json
local_map = locals()

args = json.loads(local_map['__args__'])
source = local_map['__source__']
filename = local_map.get("__filename__", "<tmp>")
# local_map['__ret__'] =render_tpl()
del local_map['__args__']
del local_map['__source__']

ret = render_tpl(source, filename, args)
locals()['__ret__'] = ret
args = None
source = None
filename = None

print(ret)