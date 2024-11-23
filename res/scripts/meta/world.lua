---@meta esprit.world

---@alias Tile "Floor"|"Wall"|"Exit"|

local world = {}

--- Return all characters in the world.
---@return [Piece]
function world.characters() end

--- Return the character at the given position.
---@param x integer
---@return Piece?
function world.character_at(x, y) end

--- Return the characters within the given radius of the given position.
---@param x integer
---@param y integer
---@param radius integer A radius of 0 represents only the tile at x, y.
---@return [Piece]
function world.characters_within(x, y, radius) end

--- Returns the tile at the given position.
---@param x integer
---@param y integer
---@return Tile?
function world.tile(x, y) end

return world
