---@module "lib.consider.attack"
local combat = require "combat";
local world = require "world";

local considerations = {}

for _, character in ipairs(world.characters_within(User.x, User.y, 1)) do
	if not combat.alliance_check(User, character) then
		table.insert(considerations, {
			arguments = { target = character },
			heuristics = {
				Heuristic.damage(
					character,
					Magnitude - character.stats.defense
				),
				-- Estimate the drawback of close combat
				Heuristic.debuff(User, 2)
			}
		})
	end
end

return considerations
