---@module "lib.spell"
local world = require "world"

local target = world.target(User.x, User.y, Parameters.range)

local is_energy = world.prompt("Major (Y: Energy, N: Harmony)")
local first_major
if is_energy then
	first_major = world.prompt("Energy (Y: Positive, N: Negative)")
else
	first_major = world.prompt("Harmony (Y: Chaos, N: Order)")
end
local has_minor = world.prompt("Configure Minor?")

local id
local name

if has_minor then
	local first_minor
	if not is_energy then
		first_minor = world.prompt("Energy (Y: Positive, N: Negative)")
	else
		first_minor = world.prompt("Harmony (Y: Chaos, N: Order)")
	end

	if is_energy then
		if first_major then
			if first_minor then
				id = 1
				name = "Positive Chaos"
			else
				id = 2
				name = "Positive Order"
			end
		else
			if first_minor then
				id = 4
				name = "Negative Chaos"
			else
				id = 5
				name = "Negative Order"
			end
		end
	else
		if first_major then
			if first_minor then
				id = 7
				name = "Chaos Positive"
			else
				id = 8
				name = "Chaos Negative"
			end
		else
			if first_minor then
				id = 10
				name = "Order Positive"
			else
				id = 11
				name = "Order Negative"
			end
		end
	end
else
	if is_energy then
		if first_major then
			id = 0
			name = "Positive"
		else
			id = 3
			name = "Negative"
		end
	else
		if first_major then
			id = 6
			name = "Chaos"
		else
			id = 9
			name = "Order"
		end
	end
end
target:force_affinity(id);
Console:print(target:replace_nouns("{Address}'s affinity is now "..name))
