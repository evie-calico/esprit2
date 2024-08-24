---@module "lib.spell"
local combat = require "combat"
local world = require "world"

local target = world.character_at(Arguments.target.x, Arguments.target.y)
if target == nil then return end

User.sp = User.sp - Level

if not combat.alliance_check(User, target)
	and Affinity:magnitude(Parameters.magnitude) - target.stats.resistance <= 0
then
	Console:combat_log(
		combat.format(User, target, "{target_Address} resisted {self_address}'s swap."),
		Log.Miss
	)
else
	local cx, cy = User.x, User.y
	User.x = target.x
	User.y = target.y
	target.x = cx
	target.y = cy

	Console:combat_log(
		combat.format(User, target, "{self_Address} swapped positions with {target_address}."),
		Log.Success
	)
end

return Parameters.cast_time
