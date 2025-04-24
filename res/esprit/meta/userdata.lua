---@meta _

---@alias PieceNextAttack fun(self, value: integer?): integer?, string
---@alias PieceNextSpell fun(self, value: integer?): integer?, string
---@alias Attach fun(any): any

---@class (exact) Piece: userdata
---@field x integer
---@field y integer
---@field hp integer
---@field sp integer
---@field stats Stats
---@field attacks fun(self): PieceNextAttack, self
---@field spells fun(self): PieceNextSpell, self
---@field replace_nouns fun(self, s: string): string
---@field attach fun(self, key: string, value: any)
---@field component fun(self, key: string): any
---@field detach fun(self, key: string)

---@class (exact) Attack: userdata
---@field on_consider fun(user: Piece, id: string, considerations: [Consider])?
---@field use_time integer

---@class (exact) Spell: userdata
---@field level integer
---@field on_cast string
---@field on_consider string?
---@field on_input string
---@field use_time integer

---@class (exact) Component: userdata
