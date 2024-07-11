require("combat")

local considerations = ...

for i, character in ipairs(nearby_characters) do
	if not alliance_check(user, character) then
		considerations:push(
			{ target = character },
			Heuristic:damage(
				character,
				magnitude - character.stats.defense
			),
			Heuristic:debuff(character, 1)
		)
	end
end

return considerations
