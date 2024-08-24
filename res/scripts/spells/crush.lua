---@module "lib.spell"
local combat = require "combat";
local world = require "world";

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
	-- TODO: Enemies use object pronouns; fix this.
	"{Address} gently braces {them}self against the wall",
	"{Address} slides down the wall, hitting the ground unscatched",
}

Console:combat_log(User:replace_nouns(cast_messages[math.random(#cast_messages)]), Log.Success);

for _, character in ipairs(characters) do
	if math.abs(character.x - Arguments.target.x) <= Parameters.radius and math.abs(character.y - Arguments.target.y) <= Parameters.radius then
		-- we'll start with a basic rightward movement.
		for distance_traveled = 0, Affinity:magnitude(Parameters.displacement) do
			local projected_x = character.x
			local projected_y = character.y
			if Arguments.direction == "Left" then
				projected_x = projected_x - 1
			elseif Arguments.direction == "Right" then
				projected_x =
					projected_x + 1
			end
			if Arguments.direction == "Up" then
				projected_y = projected_y - 1
			elseif Arguments.direction == "Down" then
				projected_y =
					projected_y + 1
			end

			local tile = world.tile(projected_x, projected_y)
			-- TODO: This is insufficient
			if tile ~= nil and tile ~= "Wall" then
				character.x = projected_x
				character.y = projected_y
			else
				local damage, pierce_failed = combat.apply_damage_with_pierce(
					Parameters.pierce_threshold,
					Affinity:magnitude(Parameters.magnitude) + distance_traveled * 2 - character.stats.resistance
				)

				-- TODO Make messages vary based on distance travelled.
				if damage > 0 then
					character.hp = character.hp - damage
					Console:combat_log(
						character:replace_nouns(damage_messages[math.random(#damage_messages)]),
						Log.Hit(damage)
					)
				else
					Console:combat_log(
						character:replace_nouns(failure_messages[math.random(#failure_messages)]),
						pierce_failed and Log.Glance or Log.Miss
					)
				end

				-- Skip printing a neutral message
				goto printed
			end
		end

		-- This print has to happen here because it should only be shown if the character never hit a wall.
		Console:print(character:replace_nouns(neutral_messages[math.random(#neutral_messages)]))

		::printed::
	end
end

User.sp = User.sp - Level

return Parameters.cast_time
