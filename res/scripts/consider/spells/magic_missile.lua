---@module "lib.consider.spell"
local combat = require "combat"
local world = require "world"

local considerations = {}

for _, character in ipairs(world.characters { Within = {
	x = User.x,
	y = User.y,
	range = Parameters.range,
}}) do
	if not combat.alliance_check(User, character) then
		table.insert(considerations, {
			arguments = { target = character },
			heuristics = {
				Heuristic:damage(
					character,
					Affinity:magnitude(Parameters.magnitude) - character.stats.resistance
				)
			}
		})
	end
end

return considerations
