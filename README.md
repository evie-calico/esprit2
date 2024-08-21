# Esprit 2

This is a sequel to Esprit for the Game Boy.

## Project Goals
- A more in-depth character system.
- Even more dynamic cutscenes.
- Persistent worldgen. (dungeons are always the same for each seed)
- Magic system
- Freeplay roguelike mode where you can make a character and play with no story.

## Client

The client crate (what you run to play the game) is stored in the [client/](client/) directory,
while the root crate is the esprit2 engine that the client uses to run the game.

All I/O (sdl, config files, resource loading) happens in the client,
allowing the client program to be freely swapped and modified without touching the core engine.

## Lua

[res/scripts/](res/scripts/) contains Lua scripts used to define behavior for game resources.
These files have complete [lua-language-server](https://luals.github.io/) annotations,
so recommend installing it for syntax and type checking.
