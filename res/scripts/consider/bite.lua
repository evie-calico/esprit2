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
			-- Estimate the drawback of close combat
			Heuristic:debuff(user, 2)
		)
	end
end

return considerations
