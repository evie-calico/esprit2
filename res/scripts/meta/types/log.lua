---@meta esprit.types.log

---@class (exact) log: userdata
---@field Success self
---@field Miss self
---@field Glance self
---@field Hit fun(damage: integer): self
local log = {}

return log
