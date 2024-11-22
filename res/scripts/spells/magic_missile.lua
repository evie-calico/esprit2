local combat = require "esprit.combat"
local console = require "esprit.console"
local world = require "esprit.world"
local log = require "esprit.types.log"

local args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

-- TODO: see scratch
-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

local damage, pierce_failed = combat.apply_pierce(
	Parameters.pierce_threshold,
	Affinity:magnitude(Parameters.magnitude) - target.stats.resistance
)

target.hp = target.hp - damage
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
	return combat.format(User, target, table[math.random(#table)])
end

-- Avoid showing unskilled messages too often;
-- poorly made missiles are also likely to miss or be resisted.
if pierce_failed then
	console:combat_log(pick(glancing_messages), log.Glance)
elseif damage == 0 then
	if Affinity:weak() and math.random(0, 1) == 1 then
		console:combat_log(pick(unskilled_messages), log.Miss)
	else
		console:combat_log(pick(failure_messages), log.Miss)
	end
else
	console:combat_log(pick(damage_messages), log.Hit(damage))
end

return Parameters.cast_time
