local console = require "esprit.console"
local world = require "esprit.world"

local args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end
target:force_level();
console:print(target:replace_nouns("{Address}'s level increased to " .. target.level))
