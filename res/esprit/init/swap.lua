local combat = require "engine.combat"
local world = require "engine.world"
local expression = require "engine.types.expression"
local log = require "engine.types.log"
local team = require "std:team"
local resources = require "std:resources"

-- Feel free to change this value as needed, it's set to an arbitrary value to test the resistance code.
local function magnitude(user) return user.magic end
-- TODO: pick a good range value. expression ranges would allow this to vary based on magic, eg: magic / 2 (within 4 to 8)
local range = 8
-- Long cast time to punish risky swaps
local cast_time = 48

resources.spell "swap" {
	name = "Swap",
	description = "Swaps the caster's position with the target. For non-allied targets, the spell must have a magnitude greater than the target's resistance.",
	-- TODO: Swap icon
	icon = resources.texture "magic_missile.png",

	-- This perfectly matches Luvui's affinity, making it a good early game spell for her.
	energy = "positive",
	harmony = "chaos",

	level = 4,

	on_cast = function(user, spell, args)
		local console = require "runtime.console"
		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		user.sp = user.sp - spell.level

		if not team.friendly(user, target)
			and spell:affinity(user):magnitude(magnitude(user)) - target.stats.resistance <= 0
		then
			console:combat_log(
				combat.format(user, target, "{target_Address} resisted {self_address}'s swap."),
				log.Miss
			)
		else
			local cx, cy = user.x, user.y
			user.x = target.x
			user.y = target.y
			target.x = cx
			target.y = cy

			console:combat_log(
				combat.format(user, target, "{self_Address} swapped positions with {target_address}."),
				log.Success
			)
		end

		return cast_time
	end,
	-- TODO: Allow movement heuristics to apply to characters other than the considerer, allowing for an on_consider script
	on_input = function(user)
		local input = require "runtime.input"
		return {
			target = input.cursor(user.x, user.y, range)
		}
	end,
}
