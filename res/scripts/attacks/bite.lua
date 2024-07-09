require("combat")

return coroutine.create(function()
	if target == nil then
		target = coroutine.yield({ type = "Cursor", x = user.x, y = user.y, range = 1 })
	end

	if alliance_check(user, target) then return end

	-- Bite has high damage, but also a relatively high pierce threshold for a melee attack.
	local damage, pierce_failed = apply_damage_with_pierce(4, magnitude - target.stats.defense)

	-- Biting requires you to get closer to the enemy, lowering your physical defense.
	user:inflict("close_combat")

	target.hp = target.hp - damage

	damage_messages = {
		"{self_Address} bites {target_address}",
		"{self_Address} bites into {target_address}",
		"{self_Address} bites {target_address}",
		"{self_Address} sinks {self_their} teeth into {target_address}",
	}
	glance_messages = {
		"{self_Address} weakly nibbled {target_address}",
		"{self_Address} failed to grasp {target_address} with {self_their} teeth.",
	}
	failure_messages = {
		"{target_Address} narrowly dodged {self_address}'s teeth",
		"{self_Address} tried to bite {target_address} but missed",
	}

	function pick(table)
		return target:replace_prefixed_nouns(
			"target_",
			user:replace_prefixed_nouns(
				"self_",
				table[math.random(#table)]
			)
		)
	end

	if pierce_failed then
		local log = { type = "Glance" }
		Console:combat_log(pick(glance_messages), log)
	elseif damage == 0 then
		local log = { type = "Miss" }
		Console:combat_log(pick(failure_messages), log)
	else
		local log = { type = "Hit", damage = damage }
		Console:combat_log(pick(damage_messages), log)
	end

	return 12
end)
