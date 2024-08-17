require("combat")

return coroutine.create(function()
	-- It would be nice to all some filters for *requesting* a list of characters,
	-- (sort of like yielding a Cursor ActionRequest) with some sort of query language
	-- to filter entries on the rust side.
	local characters = coroutine.yield { type = "Characters" }
	local x, y = coroutine.yield {
		type = "Cursor",
		x = caster.x,
		y = caster.y,
		range = parameters.range,
		radius = parameters.radius
	}
	for i, character in ipairs(characters) do
		if math.abs(character.x - x) <= parameters.radius and math.abs(character.y - y) <= parameters.radius then
			-- we'll start with a basic rightward movement.
			for distance_traveled = 0, parameters.displacement do
				local projected_x = character.x + 1
				local projected_y = character.y
				local tile = coroutine.yield { type = "Tile", x = projected_x, y = projected_y }
				-- TODO: This is insufficient
				if tile ~= nil and tile ~= "Wall" then
					character.x = projected_x
					character.y = projected_y
				else
					local damage, pierce_failed = apply_damage_with_pierce(
						parameters.pierce_threshold,
						affinity:magnitude(parameters.magnitude) + distance_traveled * 2 - character.stats.resistance
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

	caster.sp = caster.sp - level

	return parameters.cast_time
end)
