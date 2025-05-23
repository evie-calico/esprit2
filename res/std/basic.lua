local movement = require "std:movement"
local teams = require "std:teams"

---@param user Piece
return function(user)
	local resources = require "runtime.resources"

	---@type [Consider]
	local considerations = {}

	movement(user, considerations)

	for _, ability_id in user:abilities() do
		local ability = resources:ability(ability_id)
		if ability.on_consider ~= nil then
			ability.on_consider(user, ability_id, considerations)
		end
	end

	local risk_averse = false

	local function correct_risk(score, risky)
		if risky then if risk_averse then return -score else return 0 end end
		return score
	end

	---@param heuristic DamageHeuristic
	---@return integer
	local function damage_score(heuristic)
		local damages_ally = teams.friendly(user, heuristic.target)
		local score = heuristic.amount
		if heuristic.target.hp - heuristic.amount <= 0 then
			-- huge emphasis on killing
			score = score * 5
		end
		return correct_risk(score, damages_ally)
	end

	---@param heuristic DebuffHeuristic
	---@return integer
	local function debuff_score(heuristic)
		local damages_ally = teams.friendly(user, heuristic.target)
		return correct_risk(
			heuristic.amount * 2, -- give debuffs some extra weight
			damages_ally
		)
	end

	---@param consider Consider
	---@param weight integer?
	---@return integer
	local function sum_heuristics(consider, weight)
		if weight == nil then weight = 1 end
		local score = 0
		for _, heuristic in ipairs(consider) do
			if heuristic:damage() then
				score = score + damage_score(heuristic --[[@as DamageHeuristic]]) * weight
			elseif heuristic:debuff() then
				score = score + debuff_score(heuristic --[[@as DebuffHeuristic]]) * weight
			end
		end
		return score
	end

	---@class Score
	---@field index integer
	---@field score integer

	---@type [Score]
	local scores = {}
	for i, consider in ipairs(considerations) do
		table.insert(scores, {
			index = i,
			score = sum_heuristics(consider),
		})
	end

	---@type Score
	local highest

	for _, x in ipairs(scores) do
		if highest == nil or x.score > highest.score then
			highest = x
		end
	end

	if highest ~= nil then return considerations[highest.index] else return nil end
end
