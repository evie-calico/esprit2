-- This library serves as a wrapper around init.resources (and related internal libraries)
-- to give them a cleaner interface.
-- Because this library is only loaded once,
-- while init.resources and friends are regularly reinitialized for each module,
-- all `require` calls must be within the function where they are used.

--- Generates pretty-looking functions that support base engine resources
--- and aux modules via special-cased fields.
---
--- This syntax works because Lua allows you to omit parentheses when calling a function with
--- either a single string literal, or a single table literal.
--- Two arguments can be passed in this way by composing a second function from the first,
--- which looks like `resources.function "identifier" { keys = values }`
--- Much nicer than `resources.function("identifier", { keys = values })`.
---
--- These functions are necessary (as opposed to `resources.function["identifier"] = { keys = values }`)
--- so that implementation-defined resources (like init.client.textures)
--- can appear to have first class support as members of the table,
--- while still existing outside of the implementation's engine resources in reality.
local function impl_modules_for_types(types)
	local resources = {}
	for _, type in ipairs(types) do
		resources[type] = function(key)
			return function(table)
				local init_resources = require "init.resources"
				local passed, init_textures = pcall(require, "init.client.textures")
				if not passed then init_textures = nil end

				for field, module in pairs { textures = init_textures } do
					if table[field] ~= nil then
						module[type][key] = table[field]
						-- delete the field in case the engine ever cares about unknown keys
						table[field] = nil
					end
				end
				init_resources[type][key] = table
			end
		end
	end
	return resources
end

local resources = impl_modules_for_types { "ability", "component", "sheet", "vault" }

--- Removes a dot (.) and any subsequent non-dot characters from the end of the string.
---
---@param path string
---@return string
local function remove_extension(path)
	-- This binding is necessary to discard match count
	-- Returning the result of gsub directly would cause this function to forward both values
	local result, _ = path:gsub("%.[^.]*$", "")
	return result
end

function resources.texture(path)
	local init_resources = require "init.resources"
	local passed, init_textures = pcall(require, "init.client.textures")
	-- TODO: Return nil instead of "" once icons are moved out of the engine and into the client.
	if not passed then return "missingno" end

	local key = remove_extension(path)
	init_textures.texture[key] = init_resources.module.path .. "/" .. path
	return init_resources.module.name .. ":" .. key
end

return resources
