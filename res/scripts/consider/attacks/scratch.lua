---@module "lib.consider.attack"
local combat = require "combat";
local world = require "world";

local considerations = {}

for _, character in ipairs(world.characters { Within = {
	x = User.x,
	y = User.y,
	range = 1
}}) do
	if not combat.alliance_check(User, character) then
		table.insert(considerations, {
			arguments = { target = character },
			heuristics = {
				Heuristic:damage(
					character,
					Magnitude - character.stats.defense
				),
				Heuristic:debuff(character, 1)
			}
		})
	end
end

return considerations
