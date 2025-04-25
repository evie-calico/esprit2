---@meta engine.types.action

---@alias Value nil|boolean|integer|number|string|table<Value, Value>

---@class (exact) Action: userdata

local action = {}

---@param time integer
---@return Action
function action.wait(time) end

---@param x integer
---@param y integer
---@return Action
function action.move(x, y) end

---@param attack string
---@param args Value
---@return Action
function action.attack(attack, args) end

---@param move string
---@param args Value
---@return Action
function action.cast(move, args) end

return action
