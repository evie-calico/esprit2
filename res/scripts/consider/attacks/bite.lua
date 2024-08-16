require("combat")

local considerations = {}

for i, character in ipairs(nearby_characters) do
	if not alliance_check(user, character) then
		table.insert(considerations, {
			arguments = { target = character },
			heuristics = {
				Heuristic:damage(
					character,
					magnitude - character.stats.defense
				),
				-- Estimate the drawback of close combat
				Heuristic:debuff(user, 2)
			}
		})
	end
end

return considerations
