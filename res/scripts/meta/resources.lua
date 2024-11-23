---@meta esprit.resources

local resources = {}

---@param this self
---@param key string
---@return Status
function resources.status(this, key) end

---@param this self
---@param key string
---@return Attack
function resources.attack(this, key) end

---@param this self
---@param key string
---@return Spell
function resources.spell(this, key) end

return resources
