local input = require "esprit.input"

return {
	target = input.cursor(User.x, User.y, Parameters.range, Parameters.radius),
	direction = input.direction("Crush in which direction?"),
}
