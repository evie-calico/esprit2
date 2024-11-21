---@module "lib.consider.spell"
local combat = require "combat"
local resources = require "resources"
local world = require "world"

local user, spell_id, considerations = ...
local spell = resources:spell(spell_id)

for _, character in ipairs(world.characters_within(user.x, user.y, spell.range)) do
	if not combat.alliance_check(user, character) then
		table.insert(
			considerations,
			Consider(
				Action.cast(
					spell_id,
					{ target = { x = character.x, y = character.y } }
				),
				{
					Heuristic.damage(
						character,
						spell:affinity(user, spell.magnitude(user.stats:as_table())) - character.stats.resistance
					),
				}
			)
		)
	end
end
