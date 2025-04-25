local world = require "engine.world"
local resources = require "std:resources"
local ability = require "esprit:ability"

resources.ability "debug/level_up" {
	name = "Level Up",
	usage = "debug",
	description = "Causes the targeted character to gain a level.",
	icon = resources.texture "dummy.png",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		local level = (target:component("esprit:level") or 0) + 1
		target:attach("esprit:level", level);
		console:print(target:replace_nouns("{Address}'s level increased to " .. level))
	end,
	on_input = function(user)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, 5)
		}
	end,
}

resources.ability "debug/possess" {
	name = "Possess",
	usage = "debug",
	description = "Makes the targetted piece controllable by the user of this spell. Removes consciousness if it's already present.",
	icon = resources.texture "dummy.png",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		if target:component(":conscious") == nil then
			console:print(target:replace_nouns("{Address} has been possessed!"))
			target:attach(":conscious")
		else
			console:print(target:replace_nouns("{Address} is thinking for {themself}."))
			target:detach(":conscious")
		end
	end,
	on_input = function(user)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, 5)
		}
	end,
}

resources.ability "debug/change_affinity" {
	name = "Change Affinity",
	usage = "debug",
	description = "Changes the target's magical affinity",
	icon = resources.texture "dummy.png",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		local name
		if args.major ~= nil then
			target:attach("esprit:major", args.major)
			name = args.major
		else
			target:detach("esprit:major")
		end
		if args.minor ~= nil then
			target:attach("esprit:minor", args.minor)
			if name == nil then
				name = "minor" .. args.minor
			else
				name = name .. " " .. args.minor
			end
		else
			target:detach("esprit:minor")
			if name ~= nil then
				name = "major" .. name
			end
		end

		if name == nil then
			console:print(target:replace_nouns("{Address}'s affinity has been erased"))
		else
			console:print(target:replace_nouns("{Address}'s affinity is now " .. name))
		end
	end,
	on_input = function(user)
		local input = require "runtime.input"

		local direction_to_affinity = {
			["Up"] = ability.spell.affinity.positive,
			["Down"] = ability.spell.affinity.negative,
			["Left"] = ability.spell.affinity.order,
			["Right"] = ability.spell.affinity.chaos,
		}

		local target = input.cursor(user.x, user.y, 5)
		local major = input.prompt("Configure Major?") and
			direction_to_affinity[input.direction("Major (H: Order, J: Negative, K: Positive, L: Chaos)")]
		local minor = input.prompt("Configure Minor?") and
			direction_to_affinity[input.direction("Minor (H: Order, J: Negative, K: Positive, L: Chaos)")]

		return {
			target = target,
			major = major,
			minor = minor,
		}
	end,
}
