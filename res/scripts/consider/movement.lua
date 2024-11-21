local world = require "world"

local user, considerations = ...

for _, v in ipairs(world.characters()) do
	if user.alliance ~= v.alliance then
		table.insert(
			considerations,
			Consider(
				Action.move(v.x, v.y),
				{ Heuristic.move(v.x, v.y) }
			)
		)
	end
end
