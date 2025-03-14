local skillset = require "engine.types.skillset"
local resources = require "std:resources"

local attacks = { "esprit:scratch", "esprit:bite" }

resources.sheet "luvui" {
	level = 1,
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
	icon = resources.texture "luvui.png",

	on_consider = "std:basic",
	nouns = {
		name = "Luvui",
		proper_name = true,
		pronouns = "female",
	},
	bases = {
		heart = 30,
		soul = 15,
		power = 3,
		defense = 6,
		magic = 6,
		resistance = 3,
	},
	growths = {
		heart = 100,
		soul = 100,
		power = 40,
		defense = 50,
		magic = 80,
		resistance = 40,
	},
	skillset = skillset("chaos", "positive"),
}

resources.sheet "aris" {
	level = 1,
	attacks = attacks,
	speed = 12,
	icon = resources.texture "aris.png",

	on_consider = "std:basic",
	nouns = {
		name = "Aris",
		proper_name = true,
		pronouns = "female",
	},
	bases = {
		heart = 30,
		soul = 10,
		power = 6,
		defense = 5,
		magic = 1,
		resistance = 7,
	},
	growths = {
		heart = 100,
		soul = 80,
		power = 80,
		defense = 75,
		magic = 60,
		resistance = 50,
	},
	skillset = skillset("negative", "chaos"),
}
