local world = require "esprit.world"
local action = require "esprit.types.action"
local consider = require "esprit.types.consider"
local heuristic = require "esprit.types.heuristic"

---@type Piece, [Consider]
local user, considerations = ...

for _, v in ipairs(world.characters()) do
	if user.alliance ~= v.alliance then
		table.insert(
			considerations,
			consider(
				action.move(v.x, v.y),
				{ heuristic.move(v.x, v.y) }
			)
		)
	end
end
