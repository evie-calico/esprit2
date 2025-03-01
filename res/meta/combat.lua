---@meta engine.combat

---@class combat
local combat = {}

--- Format a given string, replacing prefixed nouns starting with target_ and self_.
---@param user Piece
---@param target Piece
---@param s string
---@return string
function combat.format(user, target, s) end

--- Adjust the magnitude based on the pierce threshold provided.
--- Returns whether or not the magnitude was adjusted.
--- @param pierce integer
--- @param magnitude integer
--- @return integer The resulting magnitude.
--- @return boolean Whether or not the magnitude was adjusted.
function combat.apply_pierce(pierce, magnitude) end

return combat
