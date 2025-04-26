local resources = require "std:resources"

-- TODO: "things sheets used to contain"
local function luvui_prototype()
	local luvui = piece.new("esprit:luvui")

	luvui:attach("esprit:level", 1)
	luvui:attach("esprit:experience", 0)

	luvui:attach("esprit:skill/major", "chaos")
	luvui:attach("esprit:skill/minor", "positive")

	return luvui
end

resources.sheet "luvui" {
	textures = {
		icon = resources.texture "sheets/luvui.png",
	},

	abilities = {
		"esprit:scratch",
		"esprit:bite",

		"esprit:magic_missile",
		"esprit:swap",
		"esprit:crush",
		"esprit:debug/level_up",
		"esprit:debug/change_affinity",
		"esprit:debug/possess",
		"esprit:debug/frenzy"
	},

	on_consider = "std:basic",
	nouns = {
		name = "Luvui",
		proper_name = true,
		pronouns = "female",
	},
	stats = {
		heart = 30,
		soul = 15,
		power = 3,
		defense = 6,
		magic = 6,
		resistance = 3,
	},
}

-- TODO: "things sheets used to contain"
local function aris_prototype()
	local aris = piece.new("esprit:aris")

	aris:attach("esprit:level", 1)
	aris:attach("esprit:experience", 0)

	aris:attach("esprit:skill/major", "negative")
	aris:attach("esprit:skill/minor", "chaos")

	return aris
end

resources.sheet "aris" {
	textures = {
		icon = resources.texture "sheets/aris.png",
	},

	abilities = {
		"esprit:scratch",
		"esprit:bite",
	},

	on_consider = "std:basic",
	nouns = {
		name = "Aris",
		proper_name = true,
		pronouns = "female",
	},
	stats = {
		heart = 30,
		soul = 10,
		power = 6,
		defense = 5,
		magic = 1,
		resistance = 7,
	},
}
