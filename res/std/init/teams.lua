local resources = require "std:resources"

resources.component "teams" {
	name = "Teams",

	-- These functions are questionable,
	-- but the alternative is expecting users to always modify the inner list properly
	-- (or to provide functions for modifiying it in lib/team.lua)
	-- They reinterpret on_(at|de)tach as insert/remove functions when provided with a string.
	-- Providing a table opts out of this behavior (though empty tables will detach the component)

	---@param user Piece
	---@param previous string[]?
	on_attach = function(user, previous)
		local current = user:component("std:teams")
		if current ~= nil and #current == 0 then
			user:detach("std:teams")
			return
		end
		if type(current) == "string" then
			previous = previous or {}
			table.insert(previous, current)
			-- This causes a recursive call!
			-- Note that previous is not a `string`, which would cause a loop
			user:attach("std:teams", previous)
		end
	end,
	---@param user Piece
	---@param previous string[]
	---@param annotation string
	on_detach = function(user, previous, annotation)
		if annotation ~= nil then
			for i = 1, #previous do
				if previous[i] == annotation then
					table.remove(previous, i)
					break
				end
			end
			if #previous > 0 then
				user:attach("std:teams", previous)
			end
		end
	end,
}
