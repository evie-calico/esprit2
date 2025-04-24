local resources = require "std:resources"
local stats = require "engine.types.stats"

resources.component "level" {
	name = "Level",
}

resources.component "major" {
	name = "Major Skill",
}

resources.component "minor" {
	name = "Minor Skill",
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
