local input = require "esprit.input"

return {
	target = input.cursor(User.x, User.y, 1, 0)
}
