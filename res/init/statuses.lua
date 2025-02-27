local component = require "esprit.resources.component"
local duration = require "esprit.types.duration"
local stats = require "esprit.types.stats"

-- TODO: This should be an engine-internal resource (hence the _ namespace)
-- TODO: This should be associated with a Value that denotes the owning player.
component "_:conscious" {
	name = "Conscious",
	icon = "dummy",
}

-- TODO: This should be an engine-internal resource (hence the _ namespace)
-- TODO: This should be associated with a Value that denotes the team's identifier. (eg, _:players, esprit:rats, etc.)
component "_:team" {
	name = "Teams",
	icon = "dummy",
}

component "bleed" {
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

component "close_combat" {
	name = "Close Combat",
	icon = "dummy",
	duration = duration.turn,
	on_debuff = function() return stats.defense(4) end
}
