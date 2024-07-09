require("combat")

local considerations = ...

for i, character in ipairs(nearby_characters) do
	if alliance_check(caster, character) then
		considerations:damage(
			character,
			basic_magic_attack_against(character)
		)
	end
end

return considerations
