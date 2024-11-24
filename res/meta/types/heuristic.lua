---@meta esprit.types.heuristic

---@class Heuristic: userdata
---@field damage fun(self): boolean
---@field debuff fun(self): boolean

---@class (exact) DamageHeuristic: Heuristic
---@field target Piece
---@field amount integer

---@class (exact) DebuffHeuristic: Heuristic
---@field target Piece
---@field amount integer

---@class (exact) MoveHeuristic: Heuristic
---@field x integer
---@field y integer

local heuristic = {}

---@param target Piece
---@param amount integer
---@return DamageHeuristic
function heuristic.damage(target, amount) end

---@param target Piece
---@param amount integer
---@return DebuffHeuristic
function heuristic.debuff(target, amount) end

---@param x integer
---@param y integer
---@return MoveHeuristic
function heuristic.move(x, y) end

return heuristic
