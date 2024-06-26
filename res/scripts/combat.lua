function apply_damage_with_pierce(pierce_threshold, pre_damage)
	local damage = math.max(pre_damage + math.min(pierce_threshold, 0), 0)
	local pierce_failed = false
	if damage > 0 and damage <= pierce_threshold then
		pierce_failed = true
		damage = 0
	end
	return damage, pierce_failed
end
