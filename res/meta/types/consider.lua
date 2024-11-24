---@meta esprit.types.consider

---@alias ConsiderNext fun(self, value: integer?): integer?, Heuristic

---@class (exact) Consider: userdata
---@field ipairs fun(self): ConsiderNext, self

---@param action Action
---@param heuristics [Heuristic]
---@return Consider
return function(action, heuristics) end
