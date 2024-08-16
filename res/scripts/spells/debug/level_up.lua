return coroutine.create(function()
	local target = coroutine.yield({ type = "Cursor", x = caster.x, y = caster.y, range = parameters.range })
	target:force_level();
	Console:print(target:replace_nouns("{Address}'s level increased to "..target.sheet.level))
end)
