local status = require "esprit.resources.status"
local duration = require "esprit.types.duration"
local stats = require "esprit.types.stats"

status "bleed" {
	name = "Bleeding",
	icon = "dummy",
	duration = duration.rest,
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

status "close_combat" {
	name = "Close Combat",
	icon = "dummy",
	duration = duration.turn,
	on_debuff = function() return stats.defense(4) end
}
