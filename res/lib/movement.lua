local world = require "engine.world"
local action = require "engine.types.action"
local consider = require "engine.types.consider"
local heuristic = require "engine.types.heuristic"
local team = require "team"

---@param user Piece
---@param considerations [Consider]
return function(user, considerations)
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
