---@module "lib.consider.attack"
local combat = require "combat";
local world = require "world";

local considerations = {}

for _, character in ipairs(world.characters_within(User.x, User.y, 1)) do
	if not combat.alliance_check(User, character) then
		table.insert(
			considerations,
			Consider(
				Action.attack(
					...,
					{ target = { x = character.x, y = character.y } }
				),
				{
					Heuristic.damage(
						character,
						Magnitude - character.stats.defense
					),
					Heuristic.debuff(character, 1)
				}
			)
		)
	end
end

return considerations
