return {
	--- Request all currently loaded character pieces.
	---@return [Piece]
	characters = function()
		return coroutine.yield({ type = "Characters" })
	end,

	character_at = function(x, y)
		assert(x, "missing x position")
		assert(y, "missing y position")
		return coroutine.yield({
			type = "Characters",
			query = {
				Within = {
					x = x,
					y = y,
					range = 0,
				}
			}
		})[1]
	end,

	characters_within = function(x, y, radius)
		return coroutine.yield({
			type = "Characters",
			query = {
				Within = {
					x = x,
					y = y,
					range = radius,
				}
			}
		})
	end,

	---@alias Tile "Wall" | "Floor" | "Exit"

	--- Request a tile from the world manager.
	---@param x integer
	---@param y integer
	---@return Tile?
	tile = function(x, y)
		return coroutine.yield({ type = "Tile", x = x, y = y })
	end,
}
