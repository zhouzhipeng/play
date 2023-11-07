from simple_template import render_tpl

local_map = locals()
# local_map['__ret__'] =render_tpl()
local_map['__ret__'] = render_tpl(local_map['__source__'],local_map.get("__filename__", "<tmp>"), local_map)
