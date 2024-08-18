---@module "lib.spell"
local combat = require "combat";
local world = require "world";

-- It would be nice to all some filters for *requesting* a list of characters,
-- (sort of like yielding a Cursor ActionRequest) with some sort of query language
-- to filter entries on the rust side.
local characters = world.characters()
local x, y = world.cursor(User.x, User.y, Parameters.range, Parameters.radius)
local direction = world.direction("Crush in which direction?")

for _, character in ipairs(characters) do
	if math.abs(character.x - x) <= Parameters.radius and math.abs(character.y - y) <= Parameters.radius then
		-- we'll start with a basic rightward movement.
		for distance_traveled = 0, Parameters.displacement do
			local projected_x = character.x
			local projected_y = character.y
			if direction == "Left" then projected_x = projected_x - 1 elseif direction == "Right" then projected_x = projected_x + 1 end
			if direction == "Up" then projected_y = projected_y - 1 elseif direction == "Down" then projected_y = projected_y + 1 end

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
						character.sheet.nouns.name.." was crushed!",
						{ type = "Hit", damage = damage }
					)
				else
					Console:combat_log(
						character.sheet.nouns.name.." resisted being squished.",
						{ type = pierce_failed and "Glance" or "Miss" }
					)
				end

				break
			end
		end
	end

end

User.sp = User.sp - Level

return Parameters.cast_time
