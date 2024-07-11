require("combat")

local considerations = ...

for i, character in ipairs(nearby_characters) do
	if not alliance_check(caster, character) then
		considerations:push(Heuristic:damage(
			character,
			basic_magic_attack_against(character)
		))
	end
end

return considerations
