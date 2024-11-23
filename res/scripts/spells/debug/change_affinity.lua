local console = require "esprit.console"
local world = require "esprit.world"

---@type Piece, Spell
local _, _, args = ...

local target = world.character_at(args.target.x, args.target.y)
if target == nil then return end

target:force_affinity(args.id);
console:print(target:replace_nouns("{Address}'s affinity is now " .. args.name))
