#!/usr/bin/env sh
set -eu

# dist-examples.sh
#
# Builds all wasm96 guest examples and collects the resulting WASM binaries into:
#   dist/examples/*.w96
#
# The .w96 extension is just a renamed .wasm file (same content), convenient for distribution.
#
# Requirements (depending on which examples you want to build):
# - Rust + wasm32-unknown-unknown target (for Rust examples)
# - zig (for C/C++ examples via zig cc/c++; and zig examples)
# - node + npm (for AssemblyScript example)
# - wabt (for wat2wasm) for WAT example
# - wasm-opt (Binaryen) for optimizing WASM binaries
#
# Usage:
#   ./scripts/dist-examples.sh
#
# Optional env vars:
#   DIST_DIR=dist/examples          # output directory (default: dist/examples)
#   RUST_TARGET=wasm32-unknown-unknown
#   RUST_PROFILE=release            # release or debug (default: release)
#   CLEAN=1                         # if set, clears dist dir before copying
#   SKIP_RUST=1|SKIP_ZIG=1|SKIP_C=1|SKIP_CPP=1|SKIP_AS=1|SKIP_WAT=1  # skip groups

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
DIST_DIR="${DIST_DIR:-dist/examples}"
RUST_TARGET="${RUST_TARGET:-wasm32-unknown-unknown}"
CARGO_FLAGS="-Z build-std=std,panic_abort"
RUST_PROFILE="${RUST_PROFILE:-release}"
RUSTFLAGS="-C target-feature=+simd128 -Zlocation-detail=none -Zfmt-debug=none"

log() {
  printf '%s\n' "dist-examples: $*"
}

warn() {
  printf '%s\n' "dist-examples: WARNING: $*" >&2
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

ensure_dir() {
  mkdir -p "$1"
}

copy_wasm_as_w96() {
  src="$1"
  name="$2"

  if [ ! -f "$src" ]; then
    warn "missing artifact: $src (skipping)"
    return 0
  fi

  ensure_dir "$DIST_DIR"
  cp "$src" "$DIST_DIR/$name.w96"
  log "wrote $DIST_DIR/$name.w96"
}

optimize_wasm_files() {
  if ! need_cmd wasm-opt; then
    warn "wasm-opt not found; skipping optimization"
    return 0
  fi

  log "optimizing WASM binaries with wasm-opt"

  for file in "$DIST_DIR"/*.w96; do
    if [ -f "$file" ]; then
      name="$(basename "$file")"
      wasm-opt "$file" -O4 -o "$file.tmp" --all-features --strip-debug
      mv "$file.tmp" "$file"
      log "optimized $name"
    fi
  done
}

# We don't rely on `realpath` being installed.
cd "$ROOT_DIR"

if [ "${CLEAN:-}" = "1" ]; then
  log "CLEAN=1 set; clearing $DIST_DIR"
  rm -rf "$DIST_DIR"
fi
ensure_dir "$DIST_DIR"

# -----------------------
# Rust examples
# -----------------------
build_rust() {
  if [ "${SKIP_RUST:-}" = "1" ]; then
    log "SKIP_RUST=1 set; skipping Rust examples"
    return 0
  fi

  if ! need_cmd cargo; then
    warn "cargo not found; skipping Rust examples"
    return 0
  fi

  # Ensure target is installed (non-fatal if rustup missing).
  if need_cmd rustup; then
    rustup target add "$RUST_TARGET" >/dev/null 2>&1 || true
  fi

  log "building Rust examples for $RUST_TARGET ($RUST_PROFILE)"

  # Use explicit package names from the workspace.
  # Note: these package names match the workspace members list in wasm96/Cargo.toml.
  cargo +nightly build $CARGO_FLAGS -p rust_guest --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest.wasm" "basic"

  cargo +nightly build $CARGO_FLAGS -p rust-guest-showcase --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_showcase.wasm" "showcase"

  cargo +nightly build $CARGO_FLAGS -p rust_guest_mp_platformer --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_mp_platformer.wasm" "platformer"

  cargo +nightly build $CARGO_FLAGS -p rust_guest_osmosis --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_osmosis.wasm" "osmosis_clone"

  cargo +nightly build $CARGO_FLAGS -p rust_guest_text --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_text.wasm" "text_demo"

  cargo +nightly build $CARGO_FLAGS -p rust-guest-3d --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_3d.wasm" "spinning_cube"

  cargo +nightly build $CARGO_FLAGS -p rust-guest-rapier --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_rapier.wasm" "throw_ball"

  cargo +nightly build $CARGO_FLAGS -p rust-guest-aseprite --target "$RUST_TARGET" --"$RUST_PROFILE"
  copy_wasm_as_w96 "target/$RUST_TARGET/$RUST_PROFILE/rust_guest_aseprite.wasm" "dwarf"

}

# -----------------------
# Zig examples
# -----------------------
build_zig() {
  if [ "${SKIP_ZIG:-}" = "1" ]; then
    log "SKIP_ZIG=1 set; skipping Zig examples"
    return 0
  fi

  if ! need_cmd zig; then
    warn "zig not found; skipping Zig examples"
    return 0
  fi

  log "building Zig examples"

  if [ -d "example/zig-guest" ]; then
    (cd "example/zig-guest" && zig build)
    copy_wasm_as_w96 "example/zig-guest/zig-out/bin/zig-guest.wasm" "pong"
  fi

  if [ -d "example/zig-guest-3d" ]; then
    (cd "example/zig-guest-3d" && zig build)
    copy_wasm_as_w96 "example/zig-guest-3d/zig-out/bin/zig-guest-3d.wasm" "obj_and_mtl"
  fi
}

# -----------------------
# C / C++ examples (zig toolchain via Makefile)
# -----------------------
build_c() {
  if [ "${SKIP_C:-}" = "1" ]; then
    log "SKIP_C=1 set; skipping C example"
    return 0
  fi

  if ! need_cmd zig; then
    warn "zig not found; skipping C example (requires zig cc)"
    return 0
  fi

  if [ ! -d "example/c-guest" ]; then
    return 0
  fi

  log "building C example"
  (cd "example/c-guest" && make clean >/dev/null 2>&1 || true && make)
  copy_wasm_as_w96 "example/c-guest/wasm96-example.wasm" "snake"
}

build_cpp() {
  if [ "${SKIP_CPP:-}" = "1" ]; then
    log "SKIP_CPP=1 set; skipping C++ example"
    return 0
  fi

  if ! need_cmd zig; then
    warn "zig not found; skipping C++ example (requires zig c++)"
    return 0
  fi

  if [ ! -d "example/cpp-guest" ]; then
    return 0
  fi

  log "building C++ example"
  (cd "example/cpp-guest" && make clean >/dev/null 2>&1 || true && make)
  copy_wasm_as_w96 "example/cpp-guest/wasm96-example.wasm" "tetris"
}

# -----------------------
# AssemblyScript example (npm)
# -----------------------
build_assemblyscript() {
  if [ "${SKIP_AS:-}" = "1" ]; then
    log "SKIP_AS=1 set; skipping AssemblyScript example"
    return 0
  fi

  if [ ! -d "example/assemblyscript-guest" ]; then
    return 0
  fi

  if ! need_cmd npm; then
    warn "npm not found; skipping AssemblyScript example"
    return 0
  fi

  log "building AssemblyScript example"
  # there is a justfile in the example, but don't assume `just` exists.
  (cd "example/assemblyscript-guest" && npm install && npm run build)

  # The example already ships a `flappy.wasm` in the repo, but we prefer the built one.
  copy_wasm_as_w96 "example/assemblyscript-guest/flappy.wasm" "flappy"
}

# -----------------------
# WAT example (wabt wat2wasm)
# -----------------------
build_wat() {
  if [ "${SKIP_WAT:-}" = "1" ]; then
    log "SKIP_WAT=1 set; skipping WAT example"
    return 0
  fi

  if [ ! -d "example/wat-guest" ]; then
    return 0
  fi

  if ! need_cmd wat2wasm; then
    warn "wat2wasm (wabt) not found; skipping WAT example"
    return 0
  fi

  log "building WAT example"
  (cd "example/wat-guest" && wat2wasm main.wat -o wat-guest.wasm)
  copy_wasm_as_w96 "example/wat-guest/wat-guest.wasm" "wat_demo"
}

# -----------------------
# Main
# -----------------------
log "output directory: $DIST_DIR"

build_rust
build_zig
build_c
build_cpp
build_assemblyscript
build_wat

optimize_wasm_files

log "done"
log "artifacts:"
ls -1 "$DIST_DIR" || true
