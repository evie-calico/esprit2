local world = require "engine.world"
local resources = require "res:resources"

resources.spell "debug/level_up" {
	name = "(DEBUG) Level Up",
	description = "Causes the targeted character to gain a level.",
	icon = resources.texture "dummy.png",

	energy = "positive",
	harmony = "order",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		target:force_level();
		console:print(target:replace_nouns("{Address}'s level increased to " .. target.level))
	end,
	on_input = require "res:input/single_target",

	parameters = { range = 5 },
}

resources.spell "debug/possess" {
	name = "(DEBUG) Possess",
	description = "Makes the targetted piece controllable by the user of this spell. Removes consciousness if it's already present.",
	icon = resources.texture "dummy.png",

	energy = "positive",
	harmony = "order",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		if target:detach(":conscious") == nil then
			console:print(target:replace_nouns("{Address} has been possessed!"))
			target:attach(":conscious")
		else
			console:print(target:replace_nouns("{Address} is thinking for {themself}."))
		end
	end,
	on_input = require "res:input/single_target",

	parameters = { range = 5 },
}

resources.spell "debug/change_affinity" {
	name = "(DEBUG) Change Affinity",
	description = "Changes the target's magical affinity",
	icon = resources.texture "dummy.png",

	energy = "positive",
	harmony = "order",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		target:force_affinity(args.id);
		console:print(target:replace_nouns("{Address}'s affinity is now " .. args.name))
	end,
	on_input = function(user, this)
		local input = require "runtime.input"
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

		local target = input.cursor(user.x, user.y, this.parameters.range)
		local is_energy = input.prompt("Major (Y: Energy, N: Harmony)")
		local first_major = input.prompt(is_energy and "Energy (Y: Positive, N: Negative)" or
			"Harmony (Y: Chaos, N: Order)")
		local id = input.prompt("Configure Minor?") and
			(input.prompt(is_energy and "Harmony (Y: Chaos, N: Order)" or "Energy (Y: Positive, N: Negative)") and
				(first_major and (is_energy and 1 or 7) or (is_energy and 4 or 10)) or
				(first_major and (is_energy and 2 or 8) or (is_energy and 5 or 11)))
			or
			(is_energy and (first_major and 0 or 3) or (first_major and 6 or 9))

		return {
			target = target,
			id = id,
			name = names[id + 1],
		}
	end,

	parameters = { range = 5 },
}
