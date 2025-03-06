return function(user)
	local input = require "runtime.input"
	return {
		target = input.cursor(user.x, user.y, 1, 0)
	}
end
