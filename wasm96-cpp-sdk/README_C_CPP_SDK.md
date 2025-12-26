# WASM96 C and C++ SDKs

This repository provides single-header SDKs for developing WASM96 applications in C and C++.

## Downloading the SDKs

You can download the latest versions of the SDKs directly from this repository:

- **C SDK**: [wasm96.h](https://raw.githubusercontent.com/sst/wasm96/main/wasm96-c-sdk/wasm96.h)
- **C++ SDK**: [wasm96.hpp](https://raw.githubusercontent.com/sst/wasm96/main/wasm96-cpp-sdk/wasm96.hpp)

Simply download the appropriate header file and include it in your project.

## Usage

### C

Include the header in your source file:

```c
#include "wasm96.h"

// Implement the required functions
void setup() {
    wasm96_graphics_set_size(640, 480);
}

void update() {
    // Update game logic here
}

void draw() {
    wasm96_graphics_background(0, 0, 0);
    wasm96_graphics_rect(100, 100, 50, 50);
}

int main() {
    // Your main function if needed
}
```

Compile with your WASM toolchain, ensuring the header functions are linked correctly.

### C++

Include the header in your source file:

```cpp
#include "wasm96.hpp"

// Implement the required functions
extern "C" void setup() {
    wasm96::Graphics::setSize(640, 480);
}

extern "C" void update() {
    // Update game logic here
}

extern "C" void draw() {
    wasm96::Graphics::background(0, 0, 0);
    wasm96::Graphics::rect(100, 100, 50, 50);
}

int main() {
    // Your main function if needed
}
```

Compile with your WASM toolchain, ensuring the header functions are linked correctly.

## API Reference

The SDKs provide access to the full WASM96 API, including graphics, input, audio, storage, and system functions. Refer to the header files for detailed function signatures and documentation.

## Building WASM

You'll need a WASM-compatible compiler toolchain. The SDKs are designed to work with any toolchain that can produce WASM modules with the expected imports and exports.