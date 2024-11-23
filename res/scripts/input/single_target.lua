local input = require "esprit.input"

local user, spell = ...

return {
	target = input.cursor(user.x, user.y, spell.range, 0)
}
