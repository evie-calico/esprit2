local expression = require "esprit.types.expression"
local team = require "team"

require "esprit.resources.attack" "scratch" {
	name = "Scratch",
	description = "Causes a small amount of bleeding damage, which reduces defense.",
	magnitude = expression "power + 4",
	use_time = 12,

	on_use = function(user, attack, args)
		local console = require "esprit.console"
		local combat = require "esprit.combat"
		local world = require "esprit.world"
		local log = require "esprit.types.log"

		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		-- TODO: Since you can't request input in the middle of a script anymore, this needs to communicate a failure reason and prompt resubmission
		-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

		local damage, pierce_failed = combat.apply_pierce(1, attack.magnitude(user.stats) - target.stats.defense)

		target.hp = target.hp - damage
		if damage > 0 or pierce_failed then
			-- Apply a small bleeding effect even if damage is 0
			-- to help weaker characters overcome their glancing blows
			-- Bleed scales up with damage because small defense losses will matter less to strong melee fighters.
			local new_magnitude = 5 + damage
			local old_magnitude = target:component("bleed") or 0
			target:attach("bleed", old_magnitude + new_magnitude)
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
			return combat.format(user, target, table[math.random(#table)])
		end

		if pierce_failed then
			console:combat_log(pick(glance_messages), log.Glance)
		elseif damage == 0 then
			console:combat_log(pick(failure_messages), log.Miss)
		else
			console:combat_log(pick(damage_messages), log.Hit(damage))
		end

		return attack.use_time
	end,
	on_consider = function(user, attack_id, considerations)
		local resources = require "esprit.resources"
		local world = require "esprit.world"
		local action = require "esprit.types.action"
		local consider = require "esprit.types.consider"
		local heuristic = require "esprit.types.heuristic"

		local attack = resources:attack(attack_id)

		for _, character in ipairs(world.characters_within(user.x, user.y, 1)) do
			if team.friendly(user, character) then
				table.insert(
					considerations,
					consider(
						action.attack(
							attack_id,
							{ target = { x = character.x, y = character.y } }
						),
						{
							heuristic.damage(
								character,
								attack.magnitude(user.stats) - character.stats.defense
							),
							heuristic.debuff(character, 1)
						}
					)
				)
			end
		end
	end,
	on_input = require "input.melee",
}
