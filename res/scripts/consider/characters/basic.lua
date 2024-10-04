local considerations = ...

local risk_averse = false

local function correct_risk(score, risky)
	if risky then if risk_averse then return -score else return 0 end end
	return score
end

local function damage_score(heuristic)
	local damages_ally = heuristic.target.alliance == User.alliance
	local score = heuristic.amount
	if heuristic.target.hp - heuristic.amount <= 0 then
		-- huge emphasis on killing
		score = score * 5
	end
	return correct_risk(score, damages_ally)
end

local function debuff_score(heuristic)
	local damages_ally = heuristic.target.alliance == User.alliance
	return correct_risk(
		heuristic.amount * 2, -- give debuffs some extra weight
		damages_ally
	)
end

local function sum_heuristics(consider, weight)
	if weight == nil then weight = 1 end
	local score = 0
	for _, heuristic in ipairs(consider.heuristics) do
		if heuristic:damage() then
			score = score + damage_score(heuristic) * weight
		elseif heuristic:debuff() then
			score = score + debuff_score(heuristic) * weight
		end
	end
	return score
end

local scores = {}
for i, consider in ipairs(considerations) do
	table.insert(scores, {
		index = i,
		score = sum_heuristics(consider),
	})
end

local highest

for _, x in ipairs(scores) do
	if highest == nil or x.score > highest.score then
		highest = x
	end
end

if highest ~= nil then return highest.index else return nil end
