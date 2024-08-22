---@module "lib.spell"
local world = require "world"

local target = world.character_at(Arguments.target.x, Arguments.target.y)
if target == nil then return end

target:force_affinity(Arguments.id);
Console:print(target:replace_nouns("{Address}'s affinity is now " .. Arguments.name))
