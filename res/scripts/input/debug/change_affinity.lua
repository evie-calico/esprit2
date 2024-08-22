---@module "lib.spell"
local world = require "world"

local names = {
	"Positive",
	"Positive Chaos",
	"Positive Order",
	"Negative",
	"Negative Chaos",
	"Negative Order",
	"Chaos",
	"Chaos Positive",
	"Chaos Negative",
	"Order",
	"Order Positive",
	"Order Negative",
}

local target = world.cursor(User.x, User.y, Parameters.range)
local is_energy = world.prompt("Major (Y: Energy, N: Harmony)")
local first_major = world.prompt(is_energy and "Energy (Y: Positive, N: Negative)" or "Harmony (Y: Chaos, N: Order)")
local id = world.prompt("Configure Minor?") and
	(world.prompt(is_energy and "Harmony (Y: Chaos, N: Order)" or "Energy (Y: Positive, N: Negative)") and
		(first_major and (is_energy and 1 or 7) or (is_energy and 4 or 10)) or
		(first_major and (is_energy and 2 or 8) or (is_energy and 5 or 11)))
	or
	(is_energy and (first_major and 0 or 3) or (first_major and 6 or 9))

return {
	target = target,
	id = id,
	name = names[id + 1],
}
