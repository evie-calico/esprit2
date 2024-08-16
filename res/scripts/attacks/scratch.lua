require("combat")

return coroutine.create(function()
	-- Prompt user for arguments if they have not been provided
	if arguments == nil then
		arguments = {
			target = coroutine.yield({ type = "Cursor", x = user.x, y = user.y, range = 1})
		}
	end

	if alliance_check(user, arguments.target) and not alliance_prompt() then return end

	local damage, pierce_failed = apply_damage_with_pierce(1, magnitude - arguments.target.stats.defense)

	arguments.target.hp = arguments.target.hp - damage
	if damage > 0 or pierce_failed then
		-- Apply a small bleeding effect even if damage is 0
		-- to help weaker characters overcome their glancing blows
		-- Bleed scales up with damage because small defense losses will matter less to strong melee fighters.
		arguments.target:inflict("bleed", 5 + damage);
	end

	damage_messages = {
		"{self_Address}'s claws rake against {target_address}",
		"{target_Address} is struck by {self_address}'s claws",
		"{self_Address} grazes {target_address} with {self_their} claws",
		"{self_Address} strikes {target_address} with {self_their} claws",
		"{self_Address} digs {self_their} claws into {target_address}",
	}
	glance_messages = {
		"{target_Address} was tickled by {self_address}'s claws",
		"{self_Address}'s claws lightly slid across {target_address}",
	}
	failure_messages = {
		"{self_Address}'s claws missed {target_address}",
		"{self_Address} barely missed {target_address} with {self_their} claws",
		"{target_Address} blocked {self_address}'s attack before {self_they} could strike",
	}

	function pick(table)
		return arguments.target:replace_prefixed_nouns(
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

	return use_time
end)
