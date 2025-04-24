local skillset = require "engine.types.skillset"
local resources = require "std:resources"

local attacks = { "esprit:scratch", "esprit:bite" }

resources.sheet "luvui" {
	textures = {
		icon = resources.texture "sheets/luvui.png",
	},

	attacks = attacks,
	spells = {
		"esprit:magic_missile",
		"esprit:swap",
		"esprit:crush",
		"esprit:debug/level_up",
		"esprit:debug/change_affinity",
		"esprit:debug/possess",
		"esprit:debug/frenzy"
	},
	speed = 12,

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
	skillset = skillset("chaos", "positive"),
}

resources.sheet "aris" {
	textures = {
		icon = resources.texture "sheets/aris.png",
	},

	attacks = attacks,
	speed = 12,

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
	skillset = skillset("negative", "chaos"),
}
