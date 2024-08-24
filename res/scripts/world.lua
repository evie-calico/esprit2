return {
	---@class CursorResult
	---@field x integer
	---@field y integer

	--- Request an x, y position from the world manager.
	---@param x integer
	---@param y integer
	---@param range integer
	---@param radius integer?
	---@return CursorResult
	cursor = function(x, y, range, radius)
		x, y = coroutine.yield(Input.Cursor(x, y, range, radius))
		return { x = x, y = y }
	end,

	--- Request a boolean response from the world manager.
	---@param message string
	---@return boolean
	prompt = function(message)
		return coroutine.yield(Input.Prompt(message))
	end,

	--- Request a direction from the world manager.
	---@param message string
	---@return "Up" | "Down" | "Left" | "Right"
	direction = function(message)
		return coroutine.yield(Input.Direction(message))
	end,

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
