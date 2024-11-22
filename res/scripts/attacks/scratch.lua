local combat = require "esprit.combat"
local console = require "esprit.console"
local world = require "esprit.world"
local log = require "esprit.types.log"

local args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

-- TODO: Since you can't request input in the middle of a script anymore, this needs to communicate a failure reason and prompt resubmission
-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

local damage, pierce_failed = combat.apply_pierce(1, Magnitude - target.stats.defense)

target.hp = target.hp - damage
if damage > 0 or pierce_failed then
	-- Apply a small bleeding effect even if damage is 0
	-- to help weaker characters overcome their glancing blows
	-- Bleed scales up with damage because small defense losses will matter less to strong melee fighters.
	target:inflict("bleed", 5 + damage);
end

local damage_messages = {
	"{self_Address}'s claws rake against {target_address}",
	"{target_Address} is struck by {self_address}'s claws",
	"{self_Address} grazes {target_address} with {self_their} claws",
	"{self_Address} strikes {target_address} with {self_their} claws",
	"{self_Address} digs {self_their} claws into {target_address}",
}
local glance_messages = {
	"{target_Address} was tickled by {self_address}'s claws",
	"{self_Address}'s claws lightly slid across {target_address}",
}
local failure_messages = {
	"{self_Address}'s claws missed {target_address}",
	"{self_Address} barely missed {target_address} with {self_their} claws",
	"{target_Address} blocked {self_address}'s attack before {self_they} could strike",
}

local function pick(table)
	return combat.format(User, target, table[math.random(#table)])
end

if pierce_failed then
	console:combat_log(pick(glance_messages), log.Glance)
elseif damage == 0 then
	console:combat_log(pick(failure_messages), log.Miss)
else
	console:combat_log(pick(damage_messages), log.Hit(damage))
end

return UseTime
