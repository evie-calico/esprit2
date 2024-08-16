require("combat")

local considerations = {}

for i, character in ipairs(nearby_characters) do
	if not alliance_check(user, character) then
		table.insert(considerations, {
			parameters = { target = character },
			heuristics = {
				Heuristic:damage(
					character,
					magnitude - character.stats.defense
				),
				Heuristic:debuff(character, 1)
			}
		})
	end
end

return considerations
