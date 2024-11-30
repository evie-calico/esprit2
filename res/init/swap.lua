local expression = require "esprit.types.expression"

require "esprit.resources.spell" "swap" {
	name = "Swap",
	description = "Swaps the caster's position with the target. For non-allied targets, the spell must have a magnitude greater than the target's resistance.",
	-- TODO: Swap icon
	icon = "magic_missile",

	-- This perfectly matches Luvui's affinity, making it a good early game spell for her.
	energy = "positive",
	harmony = "chaos",

	level = 4,

	parameters = {
		-- Feel free to change this value as needed, it's set to an arbitrary value to test the resistance code.
		magnitude = expression "magic",
		-- TODO: pick a good range value. expression ranges would allow this to vary based on magic, eg: magic / 2 (within 4 to 8)
		range = 8,
		-- Long cast time to punish risky swaps
		cast_time = 48,
	},

	on_cast = function(user, spell, args)
		local combat = require "esprit.combat"
		local console = require "esprit.console"
		local world = require "esprit.world"
		local log = require "esprit.types.log"

		local target = world.character_at(args.target.x, args.target.y)
		if target == nil then return end

		user.sp = user.sp - spell.level

		if not user:is_allied(target)
			and spell:affinity(user):magnitude(spell.magnitude(user)) - target.stats.resistance <= 0
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

		return spell.cast_time
	end,
	-- TODO: Allow movement heuristics to apply to characters other than the considerer, allowing for an on_consider script
	on_input = require "input.single_target",
}