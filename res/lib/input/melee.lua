return function(user)
	local input = require "esprit.input"

	return {
		target = input.cursor(user.x, user.y, 1, 0)
	}
end
