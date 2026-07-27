set shell := ["bash", "-uc"]

default:
    @just --list

libretro-build:
    zig build -Doptimize=ReleaseSafe

check:
    zig build check

cartridge-c-build:
    make -C templates/cartridge-c

clean:
    rm -rf .zig-cache zig-out templates/cartridge-c/cartridge.wasm
