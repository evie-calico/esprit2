---@meta _

---@type Console
Console = nil

---@class Log
---@field Success userdata
---@field Glance userdata
---@field Miss userdata
---@field Hit fun (integer) userdata
Log = nil

---@class Input
---@field Cursor fun (x: integer, y: integer, range: integer, radius: integer?) userdata
---@field Prompt fun (message: string) userdata
---@field Direction fun (message: string) userdata
Input = nil
