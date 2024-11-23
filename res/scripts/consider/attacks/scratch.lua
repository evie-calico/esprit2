local resources = require "esprit.resources"
local world = require "esprit.world"
local action = require "esprit.types.action"
local consider = require "esprit.types.consider"
local heuristic = require "esprit.types.heuristic"

---@type Piece, string, [Consider]
local user, attack_id, considerations = ...
local attack = resources:attack(attack_id)

for _, character in ipairs(world.characters_within(user.x, user.y, 1)) do
	if not user:is_allied(character) then
		table.insert(
			considerations,
			consider(
				action.attack(
					attack_id,
					{ target = { x = character.x, y = character.y } }
				),
				{
					heuristic.damage(
						character,
						attack.magnitude(user.stats:as_table()) - character.stats.defense
					),
					heuristic.debuff(character, 1)
				}
			)
		)
	end
end

return considerations
