---@meta esprit.types.log

---@class (exact) Log: userdata

---@class (exact) log: userdata
---@field Success Log
---@field Miss Log
---@field Glance Log
---@field Hit fun(damage: integer): Log
local log = {}

return log
