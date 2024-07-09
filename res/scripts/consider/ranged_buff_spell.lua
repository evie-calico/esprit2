require("combat")

local considerations = ...

for i, character in ipairs(nearby_characters) do
	considerations:damage(character, 1)
end

return considerations
