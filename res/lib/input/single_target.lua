return function(user, spell)
	local input = require "esprit.input"

	return {
		target = input.cursor(user.x, user.y, spell.parameters.range, 0)
	}
end
