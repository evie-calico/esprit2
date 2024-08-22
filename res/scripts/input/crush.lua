local world = require "world"

return {
	target = world.cursor(User.x, User.y, Parameters.range, Parameters.radius),
	direction = world.direction("Crush in which direction?"),
}
