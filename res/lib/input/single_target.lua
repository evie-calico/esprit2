local input = require "engine.input"

return function(user, spell)
	return {
		target = input.cursor(user.x, user.y, spell.parameters.range, 0)
	}
end
