---@meta esprit.console

local console = {}

---@param text string
function console:print(text) end

---@param text string
function console:print_system(text) end

---@param text string
function console:print_unimportant(text) end

---@param text string
function console:print_defeat(text) end

---@param text string
function console:print_danger(text) end

---@param text string
function console:print_important(text) end

---@param text string
function console:print_special(text) end

---@param text string
---@param log Log
function console:combat_log(text, log) end

return console
