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
}
