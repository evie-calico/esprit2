local resources = require "std:resources"
local team = require "std:team"

local function magnitude(user) return user.stats.power + 8 end
local use_time = 12

resources.ability "bite" {
	name = "Bite",
	description = "Lowers your defense until your next turn.",

	on_use = function(user, _, args)
		local combat = require "engine.combat"
		local console = require "runtime.console"
		local world = require "engine.world"
		local log = require "engine.types.log"

		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		-- TODO: see scratch.lua for info
		-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

		-- Bite has high damage, but also a relatively high pierce threshold for a melee attack.
		local damage, pierce_failed = combat.apply_pierce(4, magnitude(user) - target.stats.defense)

		-- Biting requires you to get closer to the enemy, lowering your physical defense.
		user:attach("esprit:close_combat")

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

		return use_time
	end,
	on_consider = function(user, attack_id, considerations)
		local world = require "engine.world"
		local action = require "engine.types.action"
		local consider = require "engine.types.consider"
		local heuristic = require "engine.types.heuristic"

		for _, character in ipairs(world.characters_within(user.x, user.y, 1)) do
			if not team.friendly(user, character) then
				table.insert(
					considerations,
					consider(
						action.act(
							attack_id,
							{ target = { x = character.x, y = character.y } }
						),
						{
							heuristic.damage(
								character,
								magnitude(user) - character.stats.defense
							),
							-- Estimate the drawback of close combat
							heuristic.debuff(user, 2)
						}
					)
				)
			end
		end
	end,
	on_input = function(user)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, 1, 0)
		}
	end,
}
