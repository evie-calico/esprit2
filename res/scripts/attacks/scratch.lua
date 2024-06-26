local pierce_threshold = 2
local damage = math.max(magnitude - target.sheet:stats().resistance + math.min(pierce_threshold, 0), 0)
local pierce_failed = false
if damage > 0 and damage <= pierce_threshold then
	pierce_failed = true
	damage = 0
end

damage_messages = {
	"{self_Address}'s claws rake against {target_address}",
	"{target_Address} is struck by {self_address}'s claws",
	"{self_Address} grazes {target_address} with {self_their} claws",
	"{self_Address} strikes {target_address} with {self_their} claws",
	"{self_Address} digs {self_their} claws into {target_address}",
}
failure_messages = {
	"{self_Address}'s claws barely missed {target_address}",
	"{target_Address} was tickled by {self_address}'s claws",
	"{self_Address}'s claws lightly slid across {target_address}",
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
	Console:combat_log(pick(failure_messages), log)
elseif damage == 0 then
	local log = { type = "Miss" }
	Console:combat_log(pick(failure_messages), log)
else
	local log = { type = "Hit", damage = damage }
	Console:combat_log(pick(damage_messages), log)
end
