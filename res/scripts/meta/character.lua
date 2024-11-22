---@meta _

---@class (exact) Piece
---@field x integer
---@field y integer
---@field hp integer
---@field sp integer
---@field level integer
---@field alliance integer
---@field stats Stats

---@class (exact) Stats
---@field heart integer
---@field soul integer
---@field power integer
---@field defense integer
---@field magic integer
---@field resistance integer
---@field as_table fun(): StatsTable

--- This is a non-exact version of Stats represented by a Lua table.
---@class StatsTable
---@field heart integer
---@field soul integer
---@field power integer
---@field defense integer
---@field magic integer
---@field resistance integer
