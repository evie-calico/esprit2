---@meta _

---@alias PieceNextAttack fun(self, value: integer?): integer?, Attack
---@alias PieceNextSpell fun(self, value: integer?): integer?, Spell

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

---@class (exact) Stats: userdata
---@field heart integer
---@field soul integer
---@field power integer
---@field defense integer
---@field magic integer
---@field resistance integer
---@field as_table fun(): StatsTable

--- This is a non-exact version of Stats represented by a Lua table.
---@class StatsTable: table<string, integer>
---@field heart integer
---@field soul integer
---@field power integer
---@field defense integer
---@field magic integer
---@field resistance integer

---@alias Expression fun(args: table<string, integer>): integer

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
---@field [string] integer|Expression Represents the contents of the spell's parameters field.
---@field affinity fun(self, character: Piece): Affinity

---@class (exact) Affinity: userdata
---@field magnitude fun(self, magnitude: integer): integer
---@field weak fun(self): boolean
---@field average fun(self): boolean
---@field strong fun(self): boolean

---@alias ConsiderNext fun(self, value: integer?): integer?, Heuristic

---@class (exact) Consider: userdata
---@field ipairs fun(self): ConsiderNext, self

---@class Heuristic: DamageHeuristic, DebuffHeuristic
---@field damage fun(self): boolean
---@field debuff fun(self): boolean

---@class (exact) DamageHeuristic: userdata
---@field target Piece
---@field amount integer

---@class (exact) DebuffHeuristic: userdata
---@field target Piece
---@field amount integer
