--- Generates pretty-looking functions that support base engine resources
--- and aux modules via special-cased fields.
local function impl_modules_for_types(types)
	local resources = {}
	for _, type in ipairs(types) do
		resources[type] = function(key)
			return function(table)
				local init_resources = require "init.resources"
				local passed, init_textures = pcall(require, "init.client.textures")
				if not passed then init_textures = nil end

				for field, module in pairs { textures = init_textures } do
					module[type][key] = table[field]
					-- delete the field in case the engine ever cares about unknown keys
					table[field] = nil
				end
				init_resources[type][key] = table
			end
		end
	end
	return resources
end

local resources = impl_modules_for_types { "attack", "component", "sheet", "spell", "vault" }

function resources.texture(path)
	local passed, init_textures = pcall(require, "init.client.textures")
	-- TODO: nil, not ""
	if not passed then return "" end

	local key = string.gsub(path, "\\..$", "")
	init_textures.textures[key] = path
	return key
end

return resources
