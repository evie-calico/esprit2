local input = require "esprit.input"

return {
	target = input.cursor(User.x, User.y, Parameters.range, 0)
}
