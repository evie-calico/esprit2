---@module "lib.spell"
local world = require "world"

local args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

target:force_affinity(args.id);
Console:print(target:replace_nouns("{Address}'s affinity is now " .. args.name))
