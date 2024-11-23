local input = require "esprit.input"

---@type Piece, Spell
local user, spell = ...

return {
	target = input.cursor(user.x, user.y, spell.range --[[@as integer]], spell.radius --[[@as integer]]),
	direction = input.direction("Crush in which direction?"),
}
