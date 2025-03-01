---@meta init.resources.component

---@alias on_debuff fun(any): Stats

---@class ComponentTable
---@field name string
---@field icon string
---@field duration Duration
---@field on_debuff on_debuff?

---@param indentifier string
---@return fun(ComponentTable): Component
function component(indentifier) end

return component
