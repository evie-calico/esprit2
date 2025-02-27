---@meta esprit.resources

local resources = {}

---@param key string
---@return Component
function resources:component(key) end

---@param key string
---@return Attack
function resources:attack(key) end

---@param key string
---@return Spell
function resources:spell(key) end

return resources
