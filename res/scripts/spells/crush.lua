local combat = require "esprit.combat"
local console = require "esprit.console"
local world = require "esprit.world"
local log = require "esprit.types.log"

---@type Piece, Spell, table<string, any>
local user, spell, args = ...

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
	if math.abs(character.x - args.target.x) <= spell.radius and math.abs(character.y - args.target.y) <= spell.radius then
		-- we'll start with a basic rightward movement.
		for distance_traveled = 0, spell:affinity(user):magnitude(spell.displacement --[[@as integer]]) do
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
					spell.pierce_threshold --[[@as integer]],
					spell:affinity(user):magnitude(spell.magnitude(user.stats:as_table())) + distance_traveled * 2 -
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

return spell.cast_time
