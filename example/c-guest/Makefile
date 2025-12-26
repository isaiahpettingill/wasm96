CC = zig cc

# Build as freestanding WebAssembly (no WASI libc/start files), so we don't need `main`.
# wasm96 host functions are imports, so they must remain unresolved at link time.
CFLAGS = -target wasm32-freestanding -O2 -ffreestanding -fno-builtin

# NOTE: Zig's cc driver does not support passing some wasm-ld flags directly.
# We instead rely on LLD's default "allow undefined" behavior for wasm imports,
# and just export the required guest entrypoints.
LDFLAGS = -Wl,--no-entry \
          -Wl,--export=setup \
          -Wl,--export=update \
          -Wl,--export=draw

all: wasm96-example.wasm

wasm96-example.wasm: main.c wasm96.h
	$(CC) $(CFLAGS) -o $@ main.c $(LDFLAGS)

clean:
	rm -f wasm96-example.wasm
