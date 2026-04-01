# SKILLS.md — Wasm (Desktop Pulley Host)

This document captures the coding conventions, architecture patterns, and
build workflow used in this project. It serves as the authoritative reference
for maintaining and extending the codebase.

## Project Identity

| Field            | Value                                       |
| ---------------- | ------------------------------------------- |
| Crate name       | `hello-world`                               |
| Binary name      | `hello`                                     |
| Language         | Rust (2024 edition)                         |
| Target           | Native arm64 macOS (`aarch64-apple-darwin`) |
| Pulley target    | `pulley64`                                  |
| Wasmtime version | 43.0.0                                      |
| Guest target     | `wasm32-unknown-unknown`                    |
| Guest allocator  | `dlmalloc`                                  |
| Guests           | 2 (`guest1`, `guest2`)                      |

## Architecture

```
build.rs (AOT compile)
  |
  +-> guest1/src/lib.rs  --(cargo build)--> guest1.wasm
  |       |                                    |
  |       +-- guest1/wit/world.wit             +-- ComponentEncoder --> guest1.cwasm
  |
  +-> guest2/src/lib.rs  --(cargo build)--> guest2.wasm
  |       |                                    |
  |       +-- guest2/wit/world.wit             +-- ComponentEncoder --> guest2.cwasm
  |
host.rs (runtime)
  |
  +-> Engine::new(pulley64)
  +-> Component::deserialize(guest1.cwasm)  --> run() -> String
  +-> Component::deserialize(guest2.cwasm)  --> run(Option<String>) -> String
                                            --> describe() -> String
```

## Source Organization

```
Cargo.toml        # Workspace root, [[bin]] = "hello", path = "host.rs"
host.rs           # Host application — load and run guests
build.rs          # AOT pipeline: compile, encode, precompile
guest1/
  Cargo.toml      # #![no_std] crate, depends on wit-bindgen + dlmalloc
  src/lib.rs      # Guest1 component — exports run() -> string
  wit/world.wit   # WIT contract for guest1-world
guest2/
  Cargo.toml      # #![no_std] crate, depends on wit-bindgen + dlmalloc
  src/lib.rs      # Guest2 component — exports run(option<string>) -> string, describe() -> string
  wit/world.wit   # WIT contract for guest2-world
tests/
  integration.rs  # Runtime integration tests
```

## Coding Conventions

### Docstrings

Every item gets a `///` doc comment: functions, structs, enums, consts,
statics, type aliases, fields, variants — including items inside function
bodies.

### Function Size

Maximum 8 lines of code per function body. Extract helpers if exceeded.
Helper functions are defined above the caller, ordered by call sequence
(top-to-bottom reading flow).

### Naming

- No underscore prefix on Rust functions (privacy via `pub`/non-`pub`)
- Constants use `SCREAMING_SNAKE_CASE`

### Blank Lines

No blank lines between code statements inside function bodies. Follow Rust
conventions for blank lines between definitions.

## WIT Contracts

### guest1-world

```wit
package component:guest1;
world guest1-world {
    export run: func() -> string;
}
```

### guest2-world

```wit
package component:guest2;
world guest2-world {
    export run: func(name: option<string>) -> string;
    export describe: func() -> string;
}
```

## Build Pipeline

### AOT Compilation (`build.rs`)

1. `cargo build --release --target wasm32-unknown-unknown` for each guest
2. `ComponentEncoder::encode()` wraps core wasm into a component
3. `Engine::new()` with `config.target("pulley64")`
4. `engine.precompile_component()` produces `.cwasm` Pulley ELF
5. Serialized bytes written to `OUT_DIR` for `include_bytes!`

### Host Runtime (`host.rs`)

1. `build_engine()` — create Engine with `pulley64` target
2. `load_component()` — `Component::deserialize()` from embedded bytes
3. `run_guest1()` — instantiate, call `run()`, print result
4. `run_guest2(name)` — instantiate, call `run(Some(name))` + `describe()`

### Build Commands

```bash
cargo build --release        # Build host + AOT-compile guests
cargo test                   # Run integration tests
cargo run --release          # Run with default name "Pulley"
cargo run --release -- Alice # Run with custom name
```

## Testing

Integration tests in `tests/integration.rs` verify:
- Guest1 `run()` returns expected string
- Guest2 `run(Some(name))` returns formatted greeting
- Guest2 `run(None)` uses default name
- Guest2 `describe()` returns expected description

## Key Differences from Embedded Variants

| Aspect           | This project              | Embedded repos                 |
| ---------------- | ------------------------- | ------------------------------ |
| `std` support    | Full `std`                | `#![no_std]` bare-metal        |
| Pulley word size | 64-bit (`pulley64`)       | 32-bit (`pulley32`)            |
| WIT direction    | Exports only (no imports) | Imports (GPIO, UART, timing)   |
| Guests           | 2 (guest1, guest2)        | 1                              |
| Engine config    | Default settings          | Custom memory limits, no traps |
| Hardware access  | None                      | Direct register I/O            |
| Allocator        | System `malloc`           | `embedded-alloc` LLFF          |
| Binary format    | Mach-O 64-bit arm64       | ELF32 ARM Thumb-2              |

## RE.md Generation

When creating an `RE.md` (Reverse Engineering document) for this project,
the analysis covers the Mach-O binary structure, arm64 host code, Cranelift
compiler (linked but unused at runtime), Pulley interpreter dispatch loop,
embedded cwasm blobs, and host-guest call flow.

### Ghidra + G-Pulley

The Ghidra analysis walkthrough uses the
[G-Pulley](https://github.com/mytechnotalent/G-Pulley) extension to analyze
**both** the arm64 host binary and the Pulley guest bytecode in a single tool.

**G-Pulley provides:**

- Custom ELF loader that extracts the `.cwasm` blob from the host binary
- SLEIGH processor spec for Pulley 32-bit and 64-bit ISA (Wasmtime v43.0.0)
- Post-load analyzer that discovers functions, trampolines, and host calls
- Full opcode decoding for all 220 primary + 310 extended Pulley opcodes

**Install:** Download from
[G-Pulley releases](https://github.com/mytechnotalent/G-Pulley/releases).
In Ghidra: **File -> Install Extensions -> + -> select zip**. Restart.

### Key Addresses to Document

For the desktop binary, identify and document:

- `hello::main` (application entry, all helpers inlined)
- `InterpreterRef::call` (native-to-Pulley boundary)
- `Interpreter::run` (Pulley dispatch loop)
- `decode_one_extended` (extended opcode handler)
- `Vm::call_start`, `Vm::call_run`
- Cranelift top functions (`constructor_simplify`, `constructor_lower`)
- Embedded cwasm blob locations in `__const` section
