---@meta engine.input

local input = {}

---@class Position
---@field x integer
---@field y integer

---@alias Direction "Left"|"Right"|"Up"|"Down"

---@param x integer
---@param y integer
---@param range integer
---@param radius integer?
---@return Position
function input.cursor(x, y, range, radius) end

---@param message string
---@return boolean
function input.prompt(message) end

---@param message string
---@return Direction
function input.direction(message) end

return input
