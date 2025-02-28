local functions = require "esprit.resources.function"
local sheet = require "esprit.resources.sheet"
local skillset = require "esprit.types.skillset"

local attacks = { "scratch", "bite" }

functions("basic", require "basic")

sheet "luvui" {
	level = 1,
	attacks = attacks,
	spells = { "magic_missile", "swap", "crush", "debug/level_up", "debug/change_affinity", "debug/possess", "debug/frenzy" },
	speed = 12,
	icon = "luvui",

	on_consider = "basic",
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

sheet "aris" {
	level = 1,
	attacks = attacks,
	speed = 12,
	icon = "aris",

	on_consider = "basic",
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
