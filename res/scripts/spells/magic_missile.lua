---@module "lib.spell"
local combat = require "combat";
local world = require "world";

-- Prompt user for Arguments if they have not been provided
if Arguments == nil then
	Arguments = {
		target = world.target(User.x, User.y, Parameters.range)
	}
end

if combat.alliance_check(User, Arguments.target) and not combat.alliance_prompt() then return end

local damage, pierce_failed = combat.apply_damage_with_pierce(
	Parameters.pierce_threshold,
	Affinity:magnitude(Parameters.magnitude) - Arguments.target.stats.resistance
)

Arguments.target.hp = Arguments.target.hp - damage
User.sp = User.sp - Level

local damage_messages = {
	"{self_Address}'s magic missile strikes {target_address}",
	"{self_Address} fires a magic missile at {target_address}",
	"{self_Address} conjures a magic missile, targetting {target_address}",
}
-- Shown when damage <= pierce_threshold
-- Signals that an attack is very close to landing.
local glancing_messages = {
	"{self_Address}'s magic missile weakly glances against {target_address}",
	"{target_Address} barely resists {self_address}'s magic missile"
}
-- Shown when damage <= 0
local failure_messages = {
	"{self_Address}'s magic missile flies past {target_address}",
	"{target_Address} narrowly dodges {self_address}'s magic missile",
	"{target_Address} easily resists {self_address}'s magic missile"
}
-- Shown when affinity is `Weak` and damage is <= 0.
-- Give feedback that a spell is unusable specifically because of its skill requirements.
local unskilled_messages = {
	"{self_Address}'s magic missile explodes mid-flight",
	"{self_Address} summons a misshapen magic missile, veering away from the target",
	"A misfired magic missile falls to the ground in front of {self_address}",
	"{self_Address} miscasts magic missile",
}

local function pick(table)
	return Arguments.target:replace_prefixed_nouns(
		"target_",
		User:replace_prefixed_nouns(
			"self_",
			table[math.random(#table)]
		)
	)
end

-- Avoid showing unskilled messages too often;
-- poorly made missiles are also likely to miss or be resisted.
if pierce_failed then
	local log = { type = "Glance" }
	Console:combat_log(pick(glancing_messages), log)
elseif damage == 0 then
	local log = { type = "Miss" }
	if Affinity:weak() and math.random(0, 1) == 1 then
		Console:combat_log(pick(unskilled_messages), log)
	else
		Console:combat_log(pick(failure_messages), log)
	end
else
	local log = { type = "Hit", damage = damage }
	Console:combat_log(pick(damage_messages), log)
end

return Parameters.cast_time
