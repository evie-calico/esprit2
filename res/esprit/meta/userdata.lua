---@meta _

---@alias PieceNextAbility fun(self, value: integer?): integer?, string
---@alias Attach fun(any): any

---@class (exact) Piece: userdata
---@field x integer
---@field y integer
---@field hp integer
---@field sp integer
---@field stats Stats
---@field abilities fun(self): PieceNextAbility, self
---@field replace_nouns fun(self, s: string): string
---@field attach fun(self, key: string, value: any)
---@field component fun(self, key: string): any
---@field detach fun(self, key: string)

---@class (exact) Ability: userdata
---@field on_consider string?

---@class (exact) Component: userdata
