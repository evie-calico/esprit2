local spell = {}

spell.affinity = {
	positive = "positive",
	negative = "negative",
	order = "order",
	chaos = "chaos",
}

function spell.sp_usage(cost)
	return "-" .. cost .. " SP"
end

function spell.make_castable(cost, affinity)
	return function(user)
		if user.sp < cost then
			return "not enough SP"
		elseif not affinity:castable(user) then
			return "improper skills to cast"
		end
	end
end

function spell.affinity.new(first, second)
	if (first == spell.affinity.positive or first == spell.affinity.negative)
		and (second == spell.affinity.positive or second == spell.affinity.negative)
		or (first == spell.affinity.order or first == spell.affinity.chaos)
		and (second == spell.affinity.order or second == spell.affinity.chaos)
	then
		error("mixed affinities")
	end

	local affinity = (first == spell.affinity.positive or first == spell.affinity.negative) and {
		energy = first,
		harmony = second,
	} or {
		energy = second,
		harmony = first,
	}

	function affinity:castable(user)
		local major = user:component("esprit:major")
		local minor = user:component("esprit:minor")
		return (
			major == self.energy
			or major == self.harmony
			or minor == self.energy
			or minor == self.harmony
		)
	end

	function affinity:score(user)
		local major = user:component("esprit:major")
		local minor = user:component("esprit:minor")
		-- bias towards majors
		-- 4, 3, 1, 0 (100%, 75%, 25%, 0%)
		return ((major == self.energy or major == self.harmony) and 3 or 0)
			+ ((minor == self.energy or major == self.harmony) and 1 or 0)
	end

	function affinity:magnitude(user, magnitude)
		return self:score(user) * magnitude / 4
	end

	function affinity:weak(user)
		return self:score(user) <= 1
	end

	return affinity
end

return spell
