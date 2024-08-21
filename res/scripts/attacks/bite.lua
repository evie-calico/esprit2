---@module "lib.attack"
local combat = require "combat"
local world = require "world"

-- Prompt User for Arguments if they have not been provided
if Arguments == nil then
	Arguments = {
		target = world.target(User.x, User.y, 1)
	}
end

if combat.alliance_check(User, Arguments.target) and not combat.alliance_prompt() then return end

-- Bite has high damage, but also a relatively high pierce threshold for a melee attack.
local damage, pierce_failed = combat.apply_damage_with_pierce(4, Magnitude - Arguments.target.stats.defense)

-- Biting requires you to get closer to the enemy, lowering your physical defense.
User:inflict("close_combat")

Arguments.target.hp = Arguments.target.hp - damage

local damage_messages = {
	"{self_Address} bites {target_address}",
	"{self_Address} bites into {target_address}",
	"{self_Address} bites {target_address}",
	"{self_Address} sinks {self_their} teeth into {target_address}",
}
local glance_messages = {
	"{self_Address} weakly nibbled {target_address}",
	"{self_Address} failed to grasp {target_address} with {self_their} teeth.",
}
local failure_messages = {
	"{target_Address} narrowly dodged {self_address}'s teeth",
	"{self_Address} tried to bite {target_address} but missed",
}

local function pick(table)
	return combat.format(User, Arguments.target, table[math.random(#table)])
end

if pierce_failed then
	local log = { type = "Glance" }
	Console:combat_log(pick(glance_messages), log)
elseif damage == 0 then
	local log = { type = "Miss" }
	Console:combat_log(pick(failure_messages), log)
else
	local log = { type = "Hit", damage = damage }
	Console:combat_log(pick(damage_messages), log)
end

return 12
