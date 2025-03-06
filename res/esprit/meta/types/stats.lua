---@meta engine.types.stats

---@class (exact) Stats: userdata
---@field heart integer
---@field soul integer
---@field power integer
---@field defense integer
---@field magic integer
---@field resistance integer

---@class StatsTable
---@field heart integer?
---@field soul integer?
---@field power integer?
---@field defense integer?
---@field magic integer?
---@field resistance integer?

---@class stats
---@operator call(StatsTable): Stats
local stats = {}

---@param heart integer
---@return Stats
function stats.heart(heart) end

---@param soul integer
---@return Stats
function stats.soul(soul) end

---@param power integer
---@return Stats
function stats.power(power) end

---@param defense integer
---@return Stats
function stats.defense(defense) end

---@param magic integer
---@return Stats
function stats.magic(magic) end

---@param resistance integer
---@return Stats
function stats.resistance(resistance) end

return stats
