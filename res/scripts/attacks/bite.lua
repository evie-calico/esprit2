local combat = require "esprit.combat"
local console = require "esprit.console"
local world = require "esprit.world"
local log = require "esprit.types.log"

--- @type Piece, Attack, table<string, any>
local user, attack, args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

-- TODO: see scratch.lua for info
-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

-- Bite has high damage, but also a relatively high pierce threshold for a melee attack.
local damage, pierce_failed = combat.apply_pierce(4, attack.magnitude(user.stats) - target.stats.defense)

-- Biting requires you to get closer to the enemy, lowering your physical defense.
user:inflict("close_combat")

target.hp = target.hp - damage

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
	return combat.format(user, target, table[math.random(#table)])
end

if pierce_failed then
	console:combat_log(pick(glance_messages), log.Glance)
elseif damage == 0 then
	console:combat_log(pick(failure_messages), log.Miss)
else
	console:combat_log(pick(damage_messages), log.Hit(damage))
end

return 12
