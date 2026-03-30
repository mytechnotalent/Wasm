# Wasm

## WebAssembly Component Model with Pulley

> Based on [embedded-wasm](https://github.com/mytechnotalent/embedded-wasm) collection — a set of repos that explores the WebAssembly Component Model runtime (Wasmtime + Pulley interpreter) from desktop tutorials to bare-metal RP2350 embedded targets with hardware capabilities exposed through WIT.

A Rust project that runs **WebAssembly Component Model** `#![no_std]` guest components through the **Pulley interpreter** using [Wasmtime](https://github.com/bytecodealliance/wasmtime). Two guest components are compiled to `wasm32-unknown-unknown`, encoded via `ComponentEncoder`, AOT-precompiled to Pulley bytecode at build time, and deserialized at runtime by the host — the same architecture used on embedded microcontrollers like the RP2350.

<br>

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Source Files](#source-files)
- [Prerequisites](#prerequisites)
- [Building](#building)
- [Usage](#usage)
- [Testing](#testing)
- [How It Works](#how-it-works)
- [WIT Interface Contract](#wit-interface-contract)
- [Extending the Project](#extending-the-project)
- [Troubleshooting](#troubleshooting)
- [Tutorial](#tutorial)
- [License](#license)

<br>

## Overview

This project demonstrates that the WebAssembly Component Model is not limited to browsers — the same host/guest architecture runs identically on a laptop and on a bare-metal microcontroller. The host uses [Wasmtime](https://github.com/bytecodealliance/wasmtime) with the **Pulley interpreter** (a portable WebAssembly execution backend) to deserialize and run AOT-precompiled WASM components that communicate through typed WIT interfaces.

**Key properties:**

- **Pure Rust** — host and guests are 100% Rust
- **`#![no_std]` guests** — guests use `wasm32-unknown-unknown` with `dlmalloc` and `wit-bindgen`, no WASI dependency
- **Component Model** — typed WIT interfaces, not raw `extern "C"` imports
- **AOT precompilation** — `build.rs` compiles guests, encodes via `ComponentEncoder`, and precompiles to Pulley bytecode at build time
- **Pulley execution** — compiled to Pulley bytecode via `config.target("pulley64")`, portable to any CPU
- **`Component::deserialize`** — host loads precompiled artifacts via `include_bytes!`, zero runtime compilation
- **Parameterized exports** — guest2 accepts `option<string>` and returns `string` via the Component Model canonical ABI
- **Multiple guests** — two components with intentionally different WIT contracts loaded by the same host
- **Industry-standard runtime** — Wasmtime is the reference WebAssembly implementation
- **Embedded-ready** — identical architecture to [embedded-wasm-uart-rp2350](https://github.com/mytechnotalent/embedded-wasm-uart-rp2350), swap `pulley64` for `pulley32`

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                 Build Time (build.rs)                    │
│                                                          │
│  guest1/src/lib.rs -> wasm32-unknown-unknown             │
│       -> ComponentEncoder -> engine.precompile_component │
│       -> guest1.cwasm (Pulley bytecode in OUT_DIR)       │
│                                                          │
│  guest2/src/lib.rs -> wasm32-unknown-unknown             │
│       -> ComponentEncoder -> engine.precompile_component │
│       -> guest2.cwasm (Pulley bytecode in OUT_DIR)       │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│                    Host (host.rs)                        │
│                                                          │
│  ┌───────────┐  ┌─────────┐  ┌──────────────────────┐    │
│  │  Engine   │  │ Linker  │  │     Store<()>        │    │
│  │  Pulley64 │  │ <()>    │  │  (no WASI state)     │    │
│  │  CompModel│  │         │  │                      │    │
│  └─────┬─────┘  └────┬────┘  └──────────┬───────────┘    │
│        │             │                  │                │
│        v             v                  v                │
│  ┌─────────────────────────────────────────────────┐     │
│  │    Component::deserialize(include_bytes!(...))  │     │
│  │          linker.instantiate(&store, &component) │     │
│  └──────────────────────┬──────────────────────────┘     │
│                         │                                │
│         ┌───────────────┴───────────────┐                │
│         │                               │                │
│         v                               v                │
│  ┌──────────────────┐   ┌──────────────────────────┐     │
│  │ guest1.cwasm     │   │ guest2.cwasm             │     │
│  │ (#![no_std])     │   │ (#![no_std])             │     │
│  │                  │   │                          │     │
│  │ exports:         │   │ exports:                 │     │
│  │   run() -> str   │   │   run(name: opt) -> str  │     │
│  │                  │   │   describe() -> str      │     │
│  │ no WASI imports  │   │ no WASI imports          │     │
│  └──────────────────┘   └──────────────────────────┘     │
│                                                          │
│  Host prints returned strings to stdout                  │
└──────────────────────────────────────────────────────────┘
```

## Project Structure

```
Wasm/
├── host.rs            # Host binary: deserialize, instantiate, call exports
├── build.rs           # AOT pipeline: compile guests, encode, precompile to Pulley
├── Cargo.toml         # Host deps (wasmtime 43.0.0) + build-deps (wit-component)
├── tests/
│   └── integration.rs # 15 integration tests: loading, exports, return values
├── guest1/
│   ├── Cargo.toml     # Guest1 package (cdylib, wit-bindgen 0.44.0, dlmalloc)
│   ├── wit/
│   │   └── world.wit  # WIT contract: export run: func() -> string
│   └── src/
│       └── lib.rs     # Guest1 impl: #![no_std], returns "guest1 run() called"
├── guest2/
│   ├── Cargo.toml     # Guest2 package (cdylib, wit-bindgen 0.44.0, dlmalloc)
│   ├── wit/
│   │   └── world.wit  # WIT contract: export run(name), export describe
│   └── src/
│       └── lib.rs     # Guest2 impl: #![no_std], greeting with optional name
├── TUTORIAL.md        # Comprehensive line-by-line tutorial
├── README.md          # This file
└── target/            # Build artifacts
```

## Source Files

### `guest1/wit/world.wit` — WIT Interface Definition

Defines the `component:guest1` package with the `guest1-world` world. Exports a single `run` function returning a `string` — the simplest possible Component Model contract.

### `guest2/wit/world.wit` — WIT Interface Definition

Defines the `component:guest2` package with the `guest2-world` world. Exports `run` with an `option<string>` parameter returning a `string`, and `describe` returning a `string` — demonstrating rich Component Model types across the host-guest boundary.

### `guest1/src/lib.rs` — WASM Guest Component

The simplest `#![no_std]` guest component compiled to `wasm32-unknown-unknown`. Uses `wit_bindgen::generate!` to produce bindings from the WIT world and implements the `Guest` trait with a `run()` function that returns `"guest1 run() called"`. Uses `dlmalloc` as the global allocator for the canonical ABI's `cabi_realloc`.

### `guest2/src/lib.rs` — WASM Guest Component

A `#![no_std]` guest component with a richer API. Implements `Guest` with `run(name: Option<String>)` that returns a greeting using the provided name (defaulting to `"world"` via `DEFAULT_NAME`) and `describe()` that returns a short string identifying the component. Demonstrates `option<string>` parameter and `string` return types through the canonical ABI.

### `build.rs` — AOT Compilation Pipeline

Orchestrates the build-time compilation of both guest components: compiles each guest crate to `wasm32-unknown-unknown` via `cargo build`, reads the core wasm bytes, encodes them as WebAssembly components via `ComponentEncoder`, and precompiles to Pulley bytecode via `engine.precompile_component()`. Writes both `.cwasm` (AOT-precompiled) and `.component.wasm` (encoded, for tests) artifacts to `OUT_DIR`.

### `host.rs` — Host Binary

Orchestrates everything at runtime: creates an `Engine` configured for Component Model + Pulley (`pulley64`), deserializes each precompiled guest artifact via `Component::deserialize` with `include_bytes!`, builds a `Linker<()>` (no WASI required), creates a `Store<()>`, instantiates each component, and calls exports. `run_guest1` calls `run()` and returns the result string; `run_guest2` calls `run(Option<String>)` and `describe()`. Reads an optional CLI argument for the guest name (defaults to `"Pulley"`).

### `tests/integration.rs` — Integration Tests

15 tests validating both guest components end-to-end: component loading, export verification (`run`, `describe`, absence of `describe` on guest1), return value checks, describe return value, absence of WASI imports, and parameter passing (default name, custom name, exact message matching).

## Prerequisites

### Toolchain

```bash
# Rust (stable) with wasm32-unknown-unknown target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

## Building

```bash
cargo build
```

The `build.rs` script handles everything automatically:

1. Compiles `guest1` and `guest2` to `wasm32-unknown-unknown` (release mode)
2. Encodes each core wasm module as a Component Model component via `ComponentEncoder`
3. AOT-precompiles each component to Pulley bytecode via `engine.precompile_component()`
4. Writes `.cwasm` artifacts to `OUT_DIR` for `include_bytes!` in the host
5. Compiles `host.rs` with the embedded precompiled components

No separate guest build step required.

## Usage

### Run with default name

```bash
cargo run --bin hello
```

Output:

```
Building Pulley component engine...
Deserializing guest1 component...
guest1 run() called
Deserializing guest2 component...
guest2 run() called: hello, Pulley!
describe: guest2 has an extra `describe` export
Done.
```

### Run with a custom name

```bash
cargo run --bin hello -- "Kevin"
```

Output:

```
Building Pulley component engine...
Deserializing guest1 component...
guest1 run() called
Deserializing guest2 component...
guest2 run() called: hello, Kevin!
describe: guest2 has an extra `describe` export
Done.
```

The `--` separates `cargo` arguments from your program's arguments. `"Kevin"` becomes `args[1]` in `host.rs`.

## Testing

```bash
cargo test
```

Runs 15 integration tests validating:

- Component loading (guest1, guest2)
- Export contract (`run` function signatures)
- Export contract (`describe` present on guest2, absent on guest1)
- Return value verification (exact strings)
- Describe return value check
- Absence of WASI imports (guests are `#![no_std]`)
- Parameter passing (default `None` -> `"world"`, custom `Some("Pulley")`)

## How It Works

### 1. The WIT Interfaces (`guest1/wit/world.wit`, `guest2/wit/world.wit`)

Define the contract between host and guest:

**guest1:**

```wit
package component:guest1;

world guest1-world {
    export run: func() -> string;
}
```

**guest2:**

```wit
package component:guest2;

world guest2-world {
    export run: func(name: option<string>) -> string;
    export describe: func() -> string;
}
```

The host looks up exports by name and verifies signatures at runtime via `get_typed_func`. If a component's exports do not match, instantiation fails.

### 2. The WASM Guests (`guest1/src/lib.rs`, `guest2/src/lib.rs`)

Each guest uses `#![no_std]` with `wit_bindgen::generate!` to produce bindings from the WIT world and implements the `Guest` trait:

**guest1:**

```rust
#![no_std]
extern crate alloc;
use alloc::string::String;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

wit_bindgen::generate!({ world: "guest1-world", path: "wit" });

impl Guest for Component {
    fn run() -> String {
        String::from("guest1 run() called")
    }
}
```

**guest2:**

```rust
#![no_std]
extern crate alloc;

const DEFAULT_NAME: &str = "world";

impl Guest for Component {
    fn run(name: Option<String>) -> String {
        let name = name.as_deref().unwrap_or(DEFAULT_NAME);
        format!("guest2 run() called: hello, {name}!")
    }

    fn describe() -> String {
        String::from("guest2 has an extra `describe` export")
    }
}
```

No WASI, no `println!` — guests return strings through the canonical ABI. `dlmalloc` provides the heap allocator required by `cabi_realloc`.

### 3. The Build Pipeline (`build.rs`)

At `cargo build` time, the build script:

1. **Compiles** each guest crate to `wasm32-unknown-unknown` via `cargo build --release --target wasm32-unknown-unknown`
2. **Reads** the core wasm binary produced by each guest build
3. **Encodes** each core module as a WebAssembly component via `ComponentEncoder` (adds component type metadata)
4. **Precompiles** each component to Pulley bytecode via `engine.precompile_component()` (AOT compilation)
5. **Writes** `.cwasm` files to `OUT_DIR` for the host to embed via `include_bytes!`

### 4. The Host Runtime (`host.rs`)

The host executes in this sequence:

1. **`main()`** — Calls `run()`, returns `wasmtime::Result`.
2. **`parse_name()`** — Reads optional CLI argument (defaults to `"Pulley"`).
3. **`build_engine()`** — Creates Engine:
   ```
   Config::new()
     .wasm_component_model(true)   -> enable Component Model
     .target("pulley64")           -> target Pulley bytecode
   Engine::new(&config)
   ```
4. **`load_component(engine, bytes)`** — Deserializes precompiled component:
   ```
   unsafe { Component::deserialize(engine, bytes) }
   ```
5. **`run_guest1(engine, component)`** — Instantiates and calls:
   ```
   Linker::<()>::new(engine)       -> empty linker, no WASI needed
   Store::new(engine, ())           -> unit state
   linker.instantiate()             -> create Instance
   get_typed_func("run")            -> look up export
   run.call()                       -> execute via Pulley, get String
   ```
6. **`run_guest2(engine, component, name)`** — Same pattern, calls `run` and `describe`.

### 5. The Call Chain

```
main()
  -> run()
       -> parse_name()                               [CLI arg or "Pulley"]
       -> build_engine()                             [Config: pulley64 + component-model]
       -> load_component(engine, GUEST1_PRECOMPILED) [Component::deserialize]
       -> run_guest1(engine, component)
            -> Linker::<()>::new(engine)
            -> Store::new(engine, ())
            -> linker.instantiate(&store, &component)
            -> get_typed_func::<(), (String,)>("run")
            -> run.call(&store, ())                  [Pulley interprets guest bytecode]
              -> guest returns String                [via canonical ABI]
       -> load_component(engine, GUEST2_PRECOMPILED) [Component::deserialize]
       -> run_guest2(engine, component, name)
            -> get_typed_func::<(Option<String>,), (String,)>("run")
            -> run.call(&store, (Some(name),))       [Pulley interprets guest bytecode]
            -> get_typed_func::<(), (String,)>("describe")
            -> describe.call(&store, ())
```

### 6. Adding a New Component (guest3)

1. Create a new guest crate:
   ```bash
   cargo init --lib guest3
   ```

2. Configure `guest3/Cargo.toml`:
   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   dlmalloc = { version = "0.2", features = ["global"] }
   wit-bindgen = "0.44.0"

   [workspace]
   ```

3. Create `guest3/wit/world.wit`:
   ```wit
   package component:guest3;

   world guest3-world {
       export run: func() -> string;
   }
   ```

4. Implement `guest3/src/lib.rs` with `#![no_std]`, `wit_bindgen::generate!`, and the `Guest` trait.

5. Add to `build.rs` — add constants for paths and names, add a `compile_guest_to_pulley` call in `main()`.

6. Add to `host.rs` — add `GUEST3_PRECOMPILED` constant, add a `run_guest3` function, call it from `run()`.

7. Build and run:
   ```bash
   cargo build && cargo run --bin hello
   ```

## WIT Interface Contract

**guest1:**

```wit
package component:guest1;

world guest1-world {
    export run: func() -> string;
}
```

**guest2:**

```wit
package component:guest2;

world guest2-world {
    export run: func(name: option<string>) -> string;
    export describe: func() -> string;
}
```

| Component | Function   | Signature                              | Description                                            |
| --------- | ---------- | -------------------------------------- | ------------------------------------------------------ |
| guest1    | `run`      | `func() -> string`                     | Returns `"guest1 run() called"`                        |
| guest2    | `run`      | `func(name: option<string>) -> string` | Returns greeting with name (defaults to `"world"`)     |
| guest2    | `describe` | `func() -> string`                     | Returns a description string identifying the component |

## Extending the Project

### Adding New WIT Exports

1. Add the export in a guest's `world.wit`:
   ```wit
   world guest1-world {
       export run: func() -> string;
       export version: func() -> string;
   }
   ```

2. Implement the new method in `lib.rs` on the `Guest` trait.

3. Look it up in `host.rs`:
   ```rust
   let version = instance.get_typed_func::<(), (String,)>(&mut store, "version")?;
   let (v,) = version.call(&mut store, ())?;
   println!("version: {v}");
   ```

4. Rebuild (`cargo build` handles everything).

### Changing Guest Behavior

Edit the `run()` function in any guest's `lib.rs`. Run `cargo build` — the build script recompiles the guest, re-encodes, and re-precompiles automatically.

## Troubleshooting

| Symptom                                  | Cause                               | Fix                                                          |
| ---------------------------------------- | ----------------------------------- | ------------------------------------------------------------ |
| `Component::deserialize` fails           | Engine config mismatch              | Ensure runtime engine config matches build.rs config exactly |
| Build fails with guest compilation error | Missing wasm target                 | Run `rustup target add wasm32-unknown-unknown`               |
| `get_typed_func` fails                   | Signature mismatch                  | Verify WIT export matches the type parameters                |
| `config.target("pulley64")` fails        | Pulley feature not enabled          | Ensure `wasmtime` dependency has `features = ["pulley"]`     |
| Guest fails to compile                   | Missing `dlmalloc` or `wit-bindgen` | Check guest `Cargo.toml` dependencies                        |
| `cabi_realloc` link error                | No global allocator                 | Add `#[global_allocator]` with `dlmalloc::GlobalDlmalloc`    |
| Tests fail                               | Guests not rebuilt                  | Run `cargo build` before `cargo test`                        |

## Tutorial

For a comprehensive, line-by-line walkthrough of every source file, struct, and function in this project — including detailed explanations of `Engine`, `Store`, `Linker`, `Component`, AOT precompilation, Pulley, and the connection to embedded systems — see [TUTORIAL.md](TUTORIAL.md).

<br>

## License

- [MIT License](LICENSE)
