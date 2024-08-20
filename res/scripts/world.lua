return {
	--- Request an x, y position from the world manager.
    ---@param x integer
    ---@param y integer
    ---@param range integer
    ---@param radius integer
    ---@return integer
    ---@return integer
	cursor = function(x, y, range, radius)
		return coroutine.yield({ type = "Cursor", x = x, y = y, range = range, radius = radius})
	end,

    --- Request a character piece from the world manager.
    ---@param x integer
    ---@param y integer
    ---@param range integer
    ---@return Piece
    target = function(x, y, range)
		return coroutine.yield({ type = "TargetCursor", x = x, y = y, range = range})
	end,

	--- Request a boolean response from the world manager.
    ---@param message string
    ---@return boolean
	prompt = function(message)
		return coroutine.yield({ type = "Prompt", message = message })
	end,

	--- Request a direction from the world manager.
    ---@param message string
    ---@return "Up" | "Down" | "Left" | "Right"
	direction = function(message)
    	return coroutine.yield({ type = "Direction", message = message })
    end,

	--- Request all currently loaded character pieces.
    ---@return [Piece]
	characters = function()
    	return coroutine.yield({ type = "Characters" })
    end,

	characters_within = function(x, y, radius)
    	return coroutine.yield({ type = "Characters", query = {
    		x = x,
    		y = y,
    		range = radius,
    	}})
    end,

    ---@alias Tile "Wall" | "Floor" | "Exit"

    --- Request a tile from the world manager.
    ---@param x integer
    ---@param y integer
    ---@return Tile | nil
    tile = function(x, y)
    	return coroutine.yield({ type = "Tile", x = x, y = y })
    end,
}
