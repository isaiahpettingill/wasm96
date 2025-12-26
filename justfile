build-sdks:
    cargo build -p wasm96-sdk --release
    cd wasm96-kotlin-sdk && ./gradlew build
    cd wasm96-go-sdk && go build .
    cd wasm96-zig-sdk && zig build

build-core:
    cargo build -p wasm96-core --release

run_command := if os_family() == "windows" { "/c/RetroArch/retroarch.exe -L ./target/release/wasm96_core.dll" } else { "retroarch -L ./target/release/libwasm96_core.so" }

run content-path: build-core
    RUST_BACKTRACE=1 {{ run_command }} {{ content-path }} --verbose

build-example example:
    cargo build -p {{ example }} --release --target wasm32-unknown-unknown

run-rust-guest:
    just build-example rust_guest
    just run ./target/wasm32-unknown-unknown/release/rust_guest.wasm

run-rust-showcase:
    just build example rust-guest-showcase
    just run ./target/wasm32-unknown-unknown/release/rust_guest_showcase.wasm

run-rust-platformer:
    cargo build -p rust_guest_mp_platformer --release --target wasm32-unknown-unknown
    just run ./target/wasm32-unknown-unknown/release/rust_guest_mp_platformer.wasm

run-rust-3d:
    cargo build -p rust-guest-3d --release --target wasm32-unknown-unknown
    just run ./target/wasm32-unknown-unknown/release/rust_guest_3d.wasm

run-rust-rapier:
    cargo build -p rust-guest-rapier --release --target wasm32-unknown-unknown
    just run ./target/wasm32-unknown-unknown/release/rust_guest_rapier.wasm

run-rust-osmosis:
    cargo build -p rust_guest_osmosis --release --target wasm32-unknown-unknown
    just run ./target/wasm32-unknown-unknown/release/rust_guest_osmosis.wasm

# run-kotlin: Kotlin guest has compatibility issues with wasm96 core
run-kotlin:
    cd example/kotlin-guest && ./gradlew build
    just run example/kotlin-guest/build/compileSync/wasmWasi/main/productionExecutable/optimized/kotlin-guest-wasm-wasi.wasm

run-rust-text:
    cargo build -p rust_guest_text --release --target wasm32-unknown-unknown
    just run ./target/wasm32-unknown-unknown/release/rust_guest_text.wasm

run-zig-guest:
    cd example/zig-guest && zig build
    just run ./example/zig-guest/zig-out/bin/zig-guest.wasm

run-zig-guest-3d:
    cd example/zig-guest-3d && zig build
    just run ./example/zig-guest-3d/zig-out/bin/zig-guest-3d.wasm

run-v-guest-3d:
    cd example/v-guest-3d && just build
    just run ./example/v-guest-3d/v-guest-3d.wasm

run-c-guest:
    cd example/c-guest && make
    just run ./example/c-guest/wasm96-example.wasm

run-cpp-guest:
    cd example/cpp-guest && make
    just run ./example/cpp-guest/wasm96-example.wasm

run-wat-guest:
    cd example/wat-guest && wat2wasm main.wat -o wat-guest.wasm
    just run ./example/wat-guest/wat-guest.wasm

run-assemblyscript-flappy:
    cd example/assemblyscript-guest && just build
    just run ./example/assemblyscript-guest/flappy.wasm

push-v-sdk version:
    git add ./wasm96-v-sdk/v.mod || true
    git commit -m "release: version {{ version }}" || true
    git subtree push --prefix=wasm96-v-sdk origin-sdk main
    git subtree split --prefix=wasm96-v-sdk -b release-{{ version }}
    git tag {{ version }} release-{{ version }}
    git push origin-sdk {{ version }}
    git branch -D release-{{ version }}
