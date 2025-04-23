# Esprit 2

## Dependencies

The `mlua` and `sdl3` libraries provide bindings to their respective C libraries.
These libraries and the tools required to link them must be installed to build esprit2.

- sdl3
- sdl3_image
- sdl3_ttf
- luajit

In addition, the following tools *may* be required to link these programs:
- make
- pkg-config

If you have any trouble while compiling with a program not on this list, please open an issue.
It's not easy to tell what's required by the cargo build scripts for these crates.

## Client

The client crate (what you run to play the game) is stored in the [client/](client/) directory,
while the root crate is the esprit2 engine that the client uses to run the game.

All I/O (sdl, config files, resource loading) happens in the client,
allowing the client program to be freely swapped and modified without touching the core engine.

## Lua

[res/scripts/](res/scripts/) contains Lua scripts used to define behavior for game resources.
These files have complete [lua-language-server](https://luals.github.io/) annotations,
so recommend installing it for syntax and type checking.
