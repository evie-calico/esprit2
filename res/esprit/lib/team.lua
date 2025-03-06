local teams = {}

--- Returns `true` if `user` and `character` consider each other friends,
--- and `false` otherwise.
---@param user Piece
---@param character Piece
---@return boolean
function teams.friendly(user, character)
	-- You're always nice to yourself.
	if user == character then return true end

	local user_teams = user:component("esprit:teams")
	local character_teams = character:component("esprit:teams")

	-- But characters with teams should fight characters without them (nothing in common!)
	if user_teams == nil or character_teams == nil then
		return false
	end

	for user_team in ipairs(user_teams) do
		for character_team in ipairs(character_teams) do
			if user_team == character_team then return true end
		end
	end
	return false
end

return teams
