local input = require "engine.input"

return function(user)
	return {
		target = input.cursor(user.x, user.y, 1, 0)
	}
end
