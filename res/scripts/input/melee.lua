local input = require "esprit.input"

---@type Piece, Attack
local user, _ = ...

return {
	target = input.cursor(user.x, user.y, 1, 0)
}
