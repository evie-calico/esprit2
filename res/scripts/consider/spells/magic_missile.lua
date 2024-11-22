local resources = require "esprit.resources"
local world = require "esprit.world"
local action = require "esprit.types.action"
local consider = require "esprit.types.consider"
local heuristic = require "esprit.types.heuristic"

local user, spell_id, considerations = ...
local spell = resources:spell(spell_id)

for _, character in ipairs(world.characters_within(user.x, user.y, spell.range)) do
	if not user:is_allied(character) then
		table.insert(
			considerations,
			consider(
				action.cast(
					spell_id,
					{ target = { x = character.x, y = character.y } }
				),
				{
					heuristic.damage(
						character,
						spell:affinity(user, spell.magnitude(user.stats:as_table())) - character.stats.resistance
					),
				}
			)
		)
	end
end
