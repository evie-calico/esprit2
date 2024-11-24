---@meta esprit.resources.status

---@alias on_debuff fun(any): Stats

---@class StatusTable
---@field name string
---@field icon string
---@field duration Duration
---@field on_debuff on_debuff?

---@param indentifier string
---@return fun(StatusTable): Status
function status(indentifier) end

return status
