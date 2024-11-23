---@meta _

---@alias PieceNextAttack fun(self, value: integer?): integer?, string
---@alias PieceNextSpell fun(self, value: integer?): integer?, string

---@class (exact) Piece: userdata
---@field x integer
---@field y integer
---@field hp integer
---@field sp integer
---@field level integer
---@field alliance integer
---@field stats Stats
---@field attacks fun(self): PieceNextAttack, self
---@field spells fun(self): PieceNextSpell, self
---@field is_allied fun(self, other: Piece): boolean
---@field replace_nouns fun(self, s: string): string
---@field inflict fun(self, key: string, magnitude: integer?)
---@field force_affinity fun(self, id: integer) Debugging utility, not for normal use.
---@field force_level fun(self) Debugging utility, not for normal use.

---@alias Expression fun(args: userdata|table<string, integer>): integer

---@class (exact) Attack: userdata
---@field magnitude Expression
---@field on_consider string?
---@field use_time integer

---@class (exact) Spell: userdata
---@field level integer
---@field on_cast string
---@field on_consider string?
---@field on_input string
---@field use_time integer
---@field affinity fun(self, character: Piece): Affinity
---@field [string] integer|Expression Represents the contents of the spell's parameters field.

---@class (exact) Affinity: userdata
---@field magnitude fun(self, magnitude: integer): integer
---@field weak fun(self): boolean
---@field average fun(self): boolean
---@field strong fun(self): boolean

---@class (exact) Status: userdata
