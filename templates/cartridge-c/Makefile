ZIG ?= zig
TARGET := wasm32-freestanding
CFLAGS := -target $(TARGET) -O2 -ffreestanding -nostdlib -Wall -Wextra
LDFLAGS := -Wl,--no-entry -Wl,--export=wasm96_update -Wl,--initial-memory=2097152 -Wl,--max-memory=134217728

all: cartridge.wasm

cartridge.wasm: main.c wasm96.h
	$(ZIG) cc $(CFLAGS) main.c $(LDFLAGS) -o $@

clean:
	rm -f cartridge.wasm

.PHONY: all clean
