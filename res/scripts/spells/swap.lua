return coroutine.create(function()
	-- Prompt user for arguments if they have not been provided
	if arguments == nil then
		arguments = {
			target = coroutine.yield({ type = "Cursor", x = caster.x, y = caster.y, range = parameters.range})
		}
	end

	caster.sp = caster.sp - level

	function format(s)
		return arguments.target:replace_prefixed_nouns(
			"target_",
			caster:replace_prefixed_nouns(
				"self_",
				s
			)
		)
	end

	if not alliance_check(caster, arguments.target)
		and affinity:magnitude(parameters.magnitude) - arguments.target.stats.resistance <= 0
	then
		local log = { type = "Miss" }
		Console:combat_log(format("{target_Address} resisted {self_address}'s swap."), log)
	else
		local cx, cy = caster.x, caster.y
		caster.x = arguments.target.x
		caster.y = arguments.target.y
		arguments.target.x = cx
		arguments.target.y = cy

		local log = { type = "Success" }
		Console:combat_log(format("{self_Address} swapped positions with {target_address}."), log)
	end

	return parameters.cast_time
end)
