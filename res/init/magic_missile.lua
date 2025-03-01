local expression = require "engine.types.expression"
local team = require "team"

require "init.resources.spell" "magic_missile" {
	name = "Magic Missile",
	icon = "magic_missile",
	description = "Constructs a searing ray of magical energy that can be fired at a target.",

	energy = "negative",
	harmony = "order",

	level = 1,

	parameters = {
		magnitude = expression "magic + 4",
		pierce_threshold = 2,
		range = 5,
		cast_time = 12,
	},

	on_cast = function(user, spell, args)
		local combat = require "engine.combat"
		local console = require "runtime.console"
		local world = require "engine.world"
		local log = require "engine.types.log"

		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		-- TODO: see scratch
		-- if combat.alliance_check(User, target) and not combat.alliance_prompt() then return end

		local damage, pierce_failed = combat.apply_pierce(
			spell.parameters.pierce_threshold --[[@as integer]],
			spell:affinity(user):magnitude(spell.parameters.magnitude(user.stats)) - target.stats.resistance
		)

		target.hp = target.hp - damage
		user.sp = user.sp - spell.level

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
			return combat.format(user, target, table[math.random(#table)])
		end

		-- Avoid showing unskilled messages too often;
		-- poorly made missiles are also likely to miss or be resisted.
		if pierce_failed then
			console:combat_log(pick(glancing_messages), log.Glance)
		elseif damage == 0 then
			if spell.affinity:weak() and math.random(0, 1) == 1 then
				console:combat_log(pick(unskilled_messages), log.Miss)
			else
				console:combat_log(pick(failure_messages), log.Miss)
			end
		else
			console:combat_log(pick(damage_messages), log.Hit(damage))
		end

		return spell.parameters.cast_time
	end,
	on_consider = function(user, spell_id, considerations)
		local resources = require "runtime.resources"
		local world = require "engine.world"
		local action = require "engine.types.action"
		local consider = require "engine.types.consider"
		local heuristic = require "engine.types.heuristic"

		local spell = resources:spell(spell_id)

		for _, character in ipairs(world.characters_within(user.x, user.y, spell.parameters.range --[[@as integer]])) do
			if not team.friendly(user, character) then
				table.insert(
					considerations,
					consider(
						action.cast(
							spell_id,
							{ target = { x = character.x, y = character.y } }
						),
						{
							heuristic.damage(
								character,
								spell:affinity(user):magnitude(
									spell.parameters.magnitude(user.stats)
								) -
								character.stats.resistance
							),
						}
					)
				)
			end
		end
	end,
	on_input = require "input.single_target",
}
