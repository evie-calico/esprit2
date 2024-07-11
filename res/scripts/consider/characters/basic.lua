local considerations = ...
local scores = {}

local risk_averse = math.random(3) ~= 3

function correct_risk(score, risky)
	if risky then if risk_averse then return -score else return 0 end end
	return score
end

function damage_score(heuristic)
	local damages_ally = heuristic.target.alliance == user.alliance
	local score = heuristic.amount
	if heuristic.target.hp - heuristic.amount <= 0 then
		-- huge emphasis on killing
		score = score * 5
	end
	return correct_risk(score, damages_ally)
end

function debuff_score(heuristic)
	local damages_ally = heuristic.target.alliance == user.alliance
	return correct_risk(
		heuristic.amount * 2, -- give debuffs some extra weight
		damages_ally
	)
end

function sum_heuristics(consider, weight)
	if weight == nil then weight = 1 end
	local score = 0
	for i, heuristic in ipairs(consider.heuristics) do
		if heuristic:damage() then
			score = score + damage_score(heuristic) * weight
		elseif heuristic:debuff() then
			score = score + debuff_score(heuristic) * weight
		end
	end
	table.insert(scores, {
		consider = consider,
		score = score
	})
end

considerations:for_each(function(consider)
	if consider:attack() then
		sum_heuristics(consider)
	elseif consider:spell() then
		sum_heuristics(consider)
	else
		sum_heuristics(consider)
	end
end)

local highest

for i, x in ipairs(scores) do
	if highest == nil or x.score > highest.score then
		highest = x
	end
end

if highest ~= nil then return highest.consider else return nil end
