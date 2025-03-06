local resources = require "esprit:resources"
local stats = require "engine.types.stats"

resources.component "teams" {
	name = "Teams",
	---@param user Piece
	---@param previous string[]?
	on_attach = function(user, previous)
		-- This function is questionable

		local current = user:component("esprit:teams")
		if current ~= nil and #current == 0 then
			user:detach("esprit:teams")
			return
		end
		if type(current) == "string" then
			previous = previous or {}
			table.insert(previous, current)
			-- This causes a recursive call!
			-- Note that previous is not a `string`, which would cause a loop
			user:attach("esprit:teams", previous)
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
				user:attach("esprit:teams", previous)
			end
		end
	end,
}

resources.component "bleed" {
	name = "Bleeding",
	visible = true,

	---@param user Piece
	on_rest = function(user) user:detach("esprit:bleed") end,
	---@param magnitude integer
	---@return Stats
	on_debuff = function(magnitude)
		local debuff = 0
		while magnitude > (debuff + 1) * 10 do
			magnitude = magnitude - (debuff + 1) * 10;
			debuff = debuff + 1;
		end

		return stats.defense(debuff)
	end
}

resources.component "close_combat" {
	name = "Close Combat",
	visible = true,

	---@param user Piece
	on_turn = function(user) user:detach("esprit:close_combat") end,
	on_debuff = function() return stats.defense(4) end
}
