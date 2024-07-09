-- Used by basic magic spells.
function basic_magic_attack_against(target)
	return affinity:magnitude(parameters.magnitude) - target.stats.resistance
end

function apply_damage_with_pierce(pierce_threshold, pre_damage)
	local damage = math.max(pre_damage + math.min(pierce_threshold, 0), 0)
	local pierce_failed = false
	if damage > 0 and damage <= pierce_threshold then
		pierce_failed = true
		damage = 0
	end
	return damage, pierce_failed
end

function alliance_check(user, target)
	return user.alliance == target.alliance
end

function alliance_prompt()
	return coroutine.yield({ type = "Prompt", message = "Really attack your ally?"})
end
