---@module "lib.consider.attack"
local combat = require "combat";
local resources = require "resources";
local world = require "world";

local user, attack_id, considerations = ...
local attack = resources:attack(attack_id)

for _, character in ipairs(world.characters_within(user.x, user.y, 1)) do
	if not combat.alliance_check(user, character) then
		table.insert(
			considerations,
			Consider(
				Action.attack(
					attack_id,
					{ target = { x = character.x, y = character.y } }
				),
				{
					Heuristic.damage(
						character,
						attack:magnitude(user) - character.stats.defense
					),
					Heuristic.debuff(character, 1)
				}
			)
		)
	end
end

return considerations
