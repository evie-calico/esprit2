local world = require "world"

return {
	target = world.cursor(User.x, User.y, Parameters.range, 0)
}
