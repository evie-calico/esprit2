local input = require "esprit.input"

---@type Piece, Spell
local user, spell = ...

return {
	target = input.cursor(user.x, user.y, spell.range --[[@as integer]], 0)
}
