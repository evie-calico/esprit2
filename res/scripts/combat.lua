return {
	--- Adjusts damage according to a pierce threshold.
	---@param pierce_threshold integer
	---@param pre_damage integer
	---@return integer
	---@return boolean
	apply_damage_with_pierce = function(pierce_threshold, pre_damage)
		local damage = math.max(pre_damage + math.min(pierce_threshold, 0), 0)
		local pierce_failed = false
		if damage > 0 and damage <= pierce_threshold then
			pierce_failed = true
			damage = 0
		end
		return damage, pierce_failed
	end,

	--- Checks if the user and target are allies
	---@param user Piece
	---@param target Piece
	---@return boolean
	alliance_check = function(user, target)
		return user.alliance == target.alliance
	end,

	--- Asks the user if they would like to target an ally.
	---@return boolean
	alliance_prompt = function()
		return require("world").prompt("Really attack your ally?")
	end,

	--- Format a given string, replacing prefixed nouns starting with target_ and self_.
	---@param user Piece
	---@param target Piece
	---@param s string
	---@return string
	format = function(user, target, s)
		return target:replace_prefixed_nouns(
			"target_",
			user:replace_prefixed_nouns(
				"self_",
				s
			)
		)
	end,
}
