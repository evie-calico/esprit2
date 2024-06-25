local damage = math.max(magnitude - target.sheet:stats().resistance + math.min(pierce_threshold, 0), 0)
local pierce_failed = false
if damage <= pierce_threshold then
	pierce_failed = true
	damage = 0
end

target.hp -= damage
caster.sp -= level

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

local log = { Hit = {
	magnitude = magnitude,
	damage = damage,
}}

-- Avoid showing unskilled messages too often;
-- poorly made missiles are also likely to miss or be resisted.
if damage == 0 and affinity:weak() and math.random(0, 1) == 1 then
	Console:combat_log(pick(unskilled_messages), log)
elseif pierce_failed then
	Console:combat_log(pick(glancing_messages), log)
elseif damage <= 0 then
	Console:combat_log(pick(failure_messages), log)
else
	Console:combat_log(pick(damage_messages), log)
end
