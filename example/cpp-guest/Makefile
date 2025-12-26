CXX = zig c++

# Build as freestanding WebAssembly (no WASI libc/start files), so we don't need `main`.
# The wasm96 host functions are wasm imports declared in the SDK header.
CXXFLAGS = -target wasm32-freestanding -std=c++17 -O2 -ffreestanding -fno-builtin
LDFLAGS = -Wl,--no-entry \
          -Wl,--export=setup \
          -Wl,--export=update \
          -Wl,--export=draw

all: wasm96-example.wasm

wasm96-example.wasm: main.cpp wasm96.hpp
	$(CXX) $(CXXFLAGS) -o $@ main.cpp $(LDFLAGS)

clean:
	rm -f wasm96-example.wasm
