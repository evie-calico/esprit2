local combat = require "esprit.combat"
local console = require "esprit.console"
local world = require "esprit.world"
local log = require "esprit.types.log"

---@type Piece, Spell, table<string, any>
local user, spell, args = ...

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
