local world = require "engine.world"
local resources = require "std:resources"

resources.spell "debug/frenzy" {
	name = "Frenzy",
	usage = "debug",
	description = "Applies frenzy",
	icon = resources.texture "dummy.png",

	energy = "positive",
	harmony = "order",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		console:print(target:replace_nouns("{Address} has been frenzied!"))
		target:attach("esprit:frenzy", 2 * 12)
	end,
	on_input = function(user)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, 5)
		}
	end,
}

---@class Frenzy
---@field time_left number
---@field former_teams string[]

resources.component "frenzy" {
	name = "Frenzied",
	visible = true,

	---@param user Piece
	---@param previous number|Frenzy?
	on_attach = function(user, previous)
		if previous == nil then
			user:attach("esprit:frenzy", {
				time_left = user:component("esprit:frenzy"),
				former_teams = user:component("std:teams") or {},
			} --[[@as Frenzy]])
			user:detach("std:teams")
		end
	end,
	---@param user Piece
	---@param previous Frenzy
	on_detach = function(user, previous)
		-- Don't overwrite the current list, in case it changed.
		for _, v in pairs(previous.former_teams) do
			user:attach("std:teams", v)
		end
	end,
	---@param user Piece
	---@param time number
	on_turn = function(user, time)
		local console = require "runtime.console"
		---@type Frenzy
		local frenzy = user:component("esprit:frenzy")
		frenzy.time_left = frenzy.time_left - time
		if frenzy.time_left <= 0 then
			user:detach("esprit:frenzy")
			console:print(user:replace_nouns("{Address} snapped out of {their} frenzy."))
		else
			user:attach("esprit:frenzy", frenzy)
		end
	end
}
