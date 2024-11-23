---@meta esprit.types.action

---@alias Value nil|boolean|integer|number|string|table<Value, Value>

local action = {}

---@param time integer
function action.wait(time) end

---@param x integer
---@param y integer
function action.move(x, y) end

---@param attack string
---@param args Value
function action.attack(attack, args) end

---@param spell string
---@param args Value
function action.cast(spell, args) end

return action
