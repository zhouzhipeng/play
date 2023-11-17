from simple_template import render_tpl, global_cache
import json

local_map = locals()

args = json.loads(local_map['__args__'])
# local_map['__ret__'] =render_tpl()
local_map['__ret__'] = render_tpl(local_map['__source__'], local_map.get("__filename__", "<tmp>"), args)

