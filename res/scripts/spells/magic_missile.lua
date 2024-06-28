require("combat")

return coroutine.create(function()
	local target = coroutine.yield({ type = "Cursor", x = caster.x, y = caster.y })

	if caster:alliance() == target:alliance() then
		Console:print_unimportant("You cannot attack your allies.");
		return
	end

	local damage, pierce_failed = apply_damage_with_pierce(
		pierce_threshold,
		magnitude - target:stats().resistance
	)

	target.hp = target.hp - damage
	caster.sp = target.sp - level

	local damage_messages = {
		"{self_Address}'s magic missile strikes {target_address}",
		"{self_Address} fires a magic missile at {target_address}",
		"{self_Address} conjures a magic missile, targetting {target_address}",
	}
	-- Shown when damage <= pierce_threshold
	-- Signals that an attack is very close to landing.
	local glancing_messages = {
		"{self_Address}'s magic missile weakly glances against {target_address}",
		"{target_Address} barely resists {self_address}'s magic missile"
	}
	-- Shown when damage <= 0
	local failure_messages = {
		"{self_Address}'s magic missile flies past {target_address}",
		"{target_Address} narrowly dodges {self_address}'s magic missile",
		"{target_Address} easily resists {self_address}'s magic missile"
	}
	-- Shown when affinity is `Weak` and damage is <= 0.
	-- Give feedback that a spell is unusable specifically because of its skill requirements.
	local unskilled_messages = {
		"{self_Address}'s magic missile explodes mid-flight",
		"{self_Address} summons a misshapen magic missile, veering away from the target",
		"A misfired magic missile falls to the ground in front of {self_address}",
		"{self_Address} miscasts magic missile",
	}

	function pick(table)
		return target:replace_prefixed_nouns(
			"target_",
			caster:replace_prefixed_nouns(
				"self_",
				table[math.random(#table)]
			)
		)
	end

	-- Avoid showing unskilled messages too often;
	-- poorly made missiles are also likely to miss or be resisted.
	if pierce_failed then
		local log = { type = "Glance" }
		Console:combat_log(pick(glancing_messages), log)
	elseif damage == 0 then
		local log = { type = "Miss" }
		if affinity:weak() and math.random(0, 1) == 1 then
			Console:combat_log(pick(unskilled_messages), log)
		else
			Console:combat_log(pick(failure_messages), log)
		end
	else
		local log = { type = "Hit", damage = damage }
		Console:combat_log(pick(damage_messages), log)
	end
end)
