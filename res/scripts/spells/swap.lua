local combat = require "esprit.combat"
local world = require "esprit.world"
local log = require "esprit.types.log"
local console = require "esprit.console"

local args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

User.sp = User.sp - Level

if not User:is_allied(target)
	and Affinity:magnitude(Parameters.magnitude) - target.stats.resistance <= 0
then
	console:combat_log(
		combat.format(User, target, "{target_Address} resisted {self_address}'s swap."),
		log.Miss
	)
else
	local cx, cy = User.x, User.y
	User.x = target.x
	User.y = target.y
	target.x = cx
	target.y = cy

	console:combat_log(
		combat.format(User, target, "{self_Address} swapped positions with {target_address}."),
		log.Success
	)
end

return Parameters.cast_time
