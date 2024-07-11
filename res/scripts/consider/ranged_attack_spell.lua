require("combat")

local considerations = ...

for i, character in ipairs(nearby_characters) do
	if not alliance_check(caster, character) then
		considerations:push(
			{ target = character },
			Heuristic:damage(
				character,
				affinity:magnitude(parameters.magnitude) - character.stats.resistance
			)
		)
	end
end

return considerations
