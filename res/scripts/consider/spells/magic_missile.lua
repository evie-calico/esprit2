---@module "lib.consider.spell"
local combat = require "combat"
local world = require "world"

local considerations = {}

for _, character in ipairs(world.characters_within(User.x, User.y, Parameters.range)) do
	if not combat.alliance_check(User, character) then
		table.insert(
			considerations,
			Consider(
				Action.cast(
					...,
					{ target = { x = character.x, y = character.y } }
				),
				{
					Heuristic.damage(
						character,
						Affinity:magnitude(Parameters.magnitude) - character.stats.resistance
					),
				}
			)
		)
	end
end

return considerations
