local team = require "team"

---@param user Piece
---@param considerations [Consider]
return function(user, considerations)
	local world = require "esprit.world"
	local action = require "esprit.types.action"
	local consider = require "esprit.types.consider"
	local heuristic = require "esprit.types.heuristic"

	for _, v in ipairs(world.characters()) do
		if team.friendly(user, v) then
			table.insert(
				considerations,
				consider(
					action.move(v.x, v.y),
					{ heuristic.move(v.x, v.y) }
				)
			)
		end
	end
end
