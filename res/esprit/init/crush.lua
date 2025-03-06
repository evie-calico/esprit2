local combat = require "engine.combat"
local world = require "engine.world"
local expression = require "engine.types.expression"
local log = require "engine.types.log"
local resources = require "esprit:resources"

resources.spell "crush" {
	name = "Crush",
	description = "Manipulates gravity to pull targets in any direction. Targets that hit walls will recieve damage according to the spell's magnitude, plus a bonus for each tile traveled.",
	icon = resources.texture "magic_missile.png",

	-- This spell is 75% effective for luvui, making it a cheap, early utility spell with some offensive capability.
	energy = "negative",
	harmony = "chaos",

	level = 2,

	parameters = {
		-- Distance adds to this, so it's effectively magic + 2 + 2d(displacement)
		magnitude = expression "magic + 2",
		pierce_threshold = 2,
		range = 6,  -- How far away the crush can be centered
		radius = 4, -- How large the area is
		displacement = 5, -- How far targets are moved
		cast_time = 12,
	},

	on_cast = function(user, spell, args)
		local console = require "runtime.console"
		-- It would be nice to all some filters for *requesting* a list of characters,
		-- (sort of like yielding a Cursor ActionRequest) with some sort of query language
		-- to filter entries on the rust side.
		local characters = world.characters()

		local cast_messages = {
			"{Address} gestures for gravity to shift",
		}
		local damage_messages = {
			"{Address} is crushed against the wall",
			"{Address}'s body strikes the sides of the dungeon",
		}
		local neutral_messages = {
			"{Address} is caught in the pull of gravity",
			"The sway of gravity draws {address} in",
		}
		-- Shown when damage <= 0
		local failure_messages = {
			"{Address} resisted being squished",
			-- This will be correct for all pronouns except "it", which will appear as "itsself".
			"{Address} gently braces {self} against the wall",
			"{Address} slides down the wall, hitting the ground unscatched",
		}

		console:combat_log(user:replace_nouns(cast_messages[math.random(#cast_messages)]), log.Success);

		for _, character in ipairs(characters) do
			if math.abs(character.x - args.target.x) <= spell.parameters.radius and math.abs(character.y - args.target.y) <= spell.parameters.radius then
				-- we'll start with a basic rightward movement.
				for distance_traveled = 0, spell:affinity(user):magnitude(spell.parameters.displacement --[[@as integer]]) do
					local projected_x = character.x
					local projected_y = character.y
					if args.direction == "Left" then
						projected_x = projected_x - 1
					elseif args.direction == "Right" then
						projected_x =
							projected_x + 1
					end
					if args.direction == "Up" then
						projected_y = projected_y - 1
					elseif args.direction == "Down" then
						projected_y =
							projected_y + 1
					end

					local tile = world.tile(projected_x, projected_y)
					-- TODO: This is insufficient
					if tile ~= nil and not tile:wall() then
						character.x = projected_x
						character.y = projected_y
					else
						local damage, pierce_failed = combat.apply_pierce(
							spell.parameters.pierce_threshold --[[@as integer]],
							spell:affinity(user):magnitude(spell.parameters.magnitude(user.stats)) +
							distance_traveled * 2 -
							character.stats.resistance
						)

						-- TODO Make messages vary based on distance travelled.
						if damage > 0 then
							character.hp = character.hp - damage
							console:combat_log(
								character:replace_nouns(damage_messages[math.random(#damage_messages)]),
								log.Hit(damage)
							)
						else
							console:combat_log(
								character:replace_nouns(failure_messages[math.random(#failure_messages)]),
								pierce_failed and log.Glance or log.Miss
							)
						end

						-- Skip printing a neutral message
						goto printed
					end
				end

				-- This print has to happen here because it should only be shown if the character never hit a wall.
				console:print(character:replace_nouns(neutral_messages[math.random(#neutral_messages)]))

				::printed::
			end
		end

		user.sp = user.sp - spell.level

		return spell.parameters.cast_time
	end,
	-- TODO: on_consider
	on_input = function(user, spell)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, spell.parameters.range, spell.parameters.radius),
			direction = input.direction("Crush in which direction?"),
		}
	end,
}
