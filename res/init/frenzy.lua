local component = require "esprit.resources.component"
local spell = require "esprit.resources.spell"

spell "debug/frenzy" {
	name = "(DEBUG) Frenzy",
	description = "Applies frenzy",
	icon = "dummy",

	energy = "positive",
	harmony = "order",

	level = 0,

	on_cast = function(_, _, args)
		local console = require "esprit.console"
		local world = require "esprit.world"

		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end
		console:print(target:replace_nouns("{Address} has been frenzied!"))
		target:attach("frenzy", 2 * 12)
	end,
	on_input = require "input.single_target",

	parameters = { range = 5 },
}

---@class Frenzy
---@field time_left number
---@field former_teams string[]

component "frenzy" {
	name = "Frenzied",
	visible = true,

	---@param user Piece
	---@param previous number|Frenzy?
	on_attach = function(user, previous)
		if previous == nil then
			user:attach("frenzy", {
				time_left = user:component("frenzy"),
				former_teams = user:component("_:team") or {},
			} --[[@as Frenzy]])
			user:detach("_:team")
		end
	end,
	---@param user Piece
	---@param previous Frenzy
	on_detach = function(user, previous)
		-- Don't overwrite the current list, in case it changed.
		for _, v in pairs(previous.former_teams) do
			user:attach("_:team", v)
		end
	end,
	---@param user Piece
	---@param time number
	on_turn = function(user, time)
		local console = require "esprit.console"
		---@type Frenzy
		local frenzy = user:component("frenzy")
		frenzy.time_left = frenzy.time_left - time
		if frenzy.time_left <= 0 then
			user:detach("frenzy")
			console:print(user:replace_nouns("{Address} snapped out of {their} frenzy."))
		else
			user:attach("frenzy", frenzy)
		end
	end
}
