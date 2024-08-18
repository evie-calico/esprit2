---@module "lib.spell"
local world = require "world"

local target = world.target(User.x, User.y, Parameters.range)
target:force_level();
Console:print(target:replace_nouns("{Address}'s level increased to "..target.sheet.level))
