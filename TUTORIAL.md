# TUTORIAL: WebAssembly Component Model with Pulley — From Scratch

## License

SPDX-License-Identifier: MIT

Copyright (c) 2026 Kevin Thomas

---

## Who This Is For

You have never used WebAssembly. You have never heard of WIT, Component Model, or Pulley. You may or may not know Rust. This tutorial assumes nothing and explains everything. By the end, you will understand exactly what every file, every struct, every function, and every line of code in this project does, and why it matters for embedded systems.

---

## Table of Contents

1. [The Big Picture](#the-big-picture)
2. [What Is WebAssembly (Wasm)](#what-is-webassembly-wasm)
3. [What Is a Wasm Runtime](#what-is-a-wasm-runtime)
4. [What Is Wasmtime](#what-is-wasmtime)
5. [What Is Pulley](#what-is-pulley)
6. [What Is WIT](#what-is-wit)
7. [What Is the Component Model](#what-is-the-component-model)
8. [What Is no_std](#what-is-no_std)
9. [What Is AOT Precompilation](#what-is-aot-precompilation)
10. [Project Structure](#project-structure)
11. [The WIT Files — Line by Line](#the-wit-files--line-by-line)
12. [Guest1 — Line by Line](#guest1--line-by-line)
13. [Guest2 — Line by Line](#guest2--line-by-line)
14. [The Build Script — Line by Line](#the-build-script--line-by-line)
15. [The Host — Line by Line](#the-host--line-by-line)
16. [The Engine — In Depth](#the-engine--in-depth)
17. [The Linker — In Depth](#the-linker--in-depth)
18. [The Store — In Depth](#the-store--in-depth)
19. [The Component — In Depth](#the-component--in-depth)
20. [The Instance — In Depth](#the-instance--in-depth)
21. [How They All Connect](#how-they-all-connect)
22. [The Integration Tests — Line by Line](#the-integration-tests--line-by-line)
23. [How to Build and Run](#how-to-build-and-run)
24. [Why This Matters for Embedded Systems](#why-this-matters-for-embedded-systems)
25. [Glossary](#glossary)

---

## The Big Picture

This project has two guest programs and one host program. The guests are written in Rust with `#![no_std]` (no standard library), compiled to WebAssembly, and packaged as "components." At build time, a build script (`build.rs`) compiles each guest, encodes it as a WebAssembly component, and AOT-precompiles it to Pulley bytecode. The host binary embeds those precompiled artifacts, deserializes them at runtime, and calls the functions that the guests export. Guests return strings through the Component Model canonical ABI — no WASI, no `println!`, no operating system dependencies.

The entire execution flow looks like this:

```
You type: cargo build

    build.rs runs:
        |
        v
    compile_guest_wasm("guest1") -> cargo build --target wasm32-unknown-unknown
        |
        v
    read_core_wasm() -> reads the guest1.wasm bytes
        |
        v
    encode_component() -> wraps core wasm as a Component Model component
        |
        v
    engine.precompile_component() -> AOT compiles to Pulley bytecode
        |
        v
    write guest1.cwasm to OUT_DIR (repeat for guest2)

You type: cargo run --bin hello

    main() starts in host.rs
        |
        v
    build_engine() creates a Wasmtime Engine configured for Pulley
        |
        v
    load_component() deserializes precompiled guest1 from include_bytes!
        |
        v
    run_guest1() creates Linker<()> + Store<()>, instantiates, calls run()
        |
        v
    Guest returns String "guest1 run() called" -> host prints it
        |
        v
    load_component() deserializes precompiled guest2 from include_bytes!
        |
        v
    run_guest2() calls run(Some(name)) and describe()
        |
        v
    Guest returns strings -> host prints them
```

---

## What Is WebAssembly (Wasm)

WebAssembly is a binary instruction format. Think of it as a portable machine language. When you write code in C, Rust, Go, or other languages, the compiler normally produces machine code for a specific processor — x86 for Intel/AMD, ARM for phones and Raspberry Pi, etc. That machine code only runs on that one type of processor.

WebAssembly is different. The compiler produces instructions for a virtual machine instead of a real processor. That means the same compiled WebAssembly binary can run on any system that has a WebAssembly runtime, regardless of what processor that system has.

Key properties of WebAssembly:

- **Portable**: One binary runs everywhere a runtime exists.
- **Sandboxed**: A WebAssembly program cannot access your filesystem, network, or memory unless the host explicitly allows it. This is a security feature.
- **Fast**: Much faster than traditional interpreters (like Python or JavaScript interpreters) because the format is designed to be close to native machine code.

---

## What Is a Wasm Runtime

A Wasm runtime is the program that actually executes WebAssembly code. The WebAssembly binary by itself is just a file of bytes. It cannot run on its own. It needs a runtime to:

1. Read and validate the binary.
2. Compile or interpret the instructions.
3. Provide any imports the guest code needs (if any).
4. Execute the guest's exported functions when asked.

Think of it like this: a `.mp3` file does not produce sound by itself. You need a music player (the runtime) to read it and send audio to your speakers. A `.wasm` file does not execute by itself. You need a Wasm runtime to read it and execute its instructions.

---

## What Is Wasmtime

Wasmtime is one specific Wasm runtime, built by the Bytecode Alliance. It is written in Rust and is designed for production use. There are other runtimes (Wasmer, WasmEdge, etc.), but this project uses Wasmtime.

Wasmtime provides the Rust crate `wasmtime` which gives you types like `Engine`, `Store`, `Component`, and `Linker`. These are the building blocks your host program uses to load and run guest WebAssembly code.

In this project, the host program in `host.rs` uses the `wasmtime` crate directly.

---

## What Is Pulley

Pulley is a software interpreter built into Wasmtime. To understand why it exists, you need to understand how Wasmtime normally executes WebAssembly.

**Normal Wasmtime execution (on your laptop or server):**

Wasmtime uses a compiler called Cranelift to translate WebAssembly instructions into native machine code for your processor (x86-64 on most laptops, AArch64 on Apple Silicon Macs). This is fast because the result is real machine code that runs directly on your CPU.

**The problem on embedded systems:**

Embedded microcontrollers (like the RP2350 on the Raspberry Pi Pico 2) use processors that Cranelift does not support. The RP2350 uses a Cortex-M33 (ARMv8-M) processor. Cranelift cannot generate native code for it. So normal Wasmtime execution does not work.

**The Pulley solution:**

Pulley is a portable interpreter inside Wasmtime. Instead of compiling WebAssembly to native machine code, Wasmtime compiles it to Pulley bytecode — a simple instruction set that the Pulley interpreter can execute on any processor, including ones Cranelift does not support.

The tradeoff: Pulley is slower than native compilation because the Pulley interpreter has to read and execute each bytecode instruction one at a time, rather than letting the CPU execute native instructions directly. But it works everywhere, which is the entire point for embedded.

**In this project:**

We set `config.target("pulley64")` in both the build script and the host. This tells Wasmtime: "Do not compile to native machine code. Compile to Pulley bytecode instead." We use `pulley64` because we are running on a 64-bit system (your Mac). On a 32-bit embedded target like the RP2350, you would use `pulley32`.

This project runs on your laptop to teach you the concepts. The same architecture — host deserializes precompiled guest component, executes via Pulley — is exactly what runs on an embedded microcontroller like the RP2350 in the [embedded-wasm-uart-rp2350](https://github.com/mytechnotalent/embedded-wasm-uart-rp2350) project.

---

## What Is WIT

WIT stands for WebAssembly Interface Types. It is a plain-text language for defining function signatures that cross the boundary between a host and a guest.

**Why it exists:**

When the host wants to call a function inside a guest, both sides need to agree on the function's name, what parameters it takes, and what it returns. WIT is how you write that agreement down.

Think of WIT as a contract. The guest says: "I promise to export a function called `run` that returns a string." The host says: "I promise to look for a function called `run` that returns a string." If both sides follow the contract, everything works. If they disagree (the guest exports `run` returning nothing but the host expects a string), instantiation fails.

**What a WIT file looks like:**

```wit
package component:guest1;

world guest1-world {
    export run: func() -> string;
}
```

This says:

- `package component:guest1;` — This WIT file belongs to a package called `component:guest1`. The package name is just an identifier.
- `world guest1-world` — A "world" is a complete description of what a component imports and exports. The name `guest1-world` is used by the `wit_bindgen::generate!` macro to find the right world.
- `export run: func() -> string;` — The component promises to export a function called `run` that takes no parameters and returns a string.

---

## What Is the Component Model

The Component Model is a standard that sits on top of WebAssembly. Plain ("core") WebAssembly only understands numbers — integers and floats. It has no concept of strings, lists, records, or other high-level types. It also has no standard way to describe imports and exports beyond raw function signatures with numeric parameters.

The Component Model adds:

1. **Rich types**: Strings, lists, records, options, results, enums, flags, and more. When your guest function takes an `Option<String>`, the Component Model handles converting that Rust type into bytes the guest can receive.
2. **WIT-based interfaces**: Instead of raw numbered imports/exports, components use named interfaces defined in WIT files.
3. **Composition**: Multiple components can be wired together, with one component's exports satisfying another component's imports.

**In plain terms:**

Without the Component Model, you can only pass numbers between the host and guest. With the Component Model, you can pass strings, structs, options, and other complex types across the boundary, and the runtime handles all the memory layout and encoding automatically.

In this project, we enable the Component Model with `config.wasm_component_model(true)` in the host, and we use `Component::deserialize` (not `Module`) to load guest artifacts. Everything flows through the Component Model.

---

## What Is no_std

In Rust, `#![no_std]` means "do not link the standard library." The Rust standard library (`std`) provides things like `println!`, file I/O, networking, threads, and other operating system features. It requires an operating system to work.

**Why guests use no_std:**

Our guest components target `wasm32-unknown-unknown` — a WebAssembly target with no operating system. There is no filesystem, no stdout, no network. The standard library's OS-dependent features cannot work. By using `#![no_std]`, we tell the Rust compiler: "This code runs without an OS. Do not try to link OS features."

**What guests still get:**

Even without `std`, guests can use `core` (basic Rust types, traits, and math) and `alloc` (heap-allocated types like `String`, `Vec`, and `format!`). The `alloc` crate requires a global memory allocator. Since there is no OS to provide one, the guests use `dlmalloc` — a simple, portable allocator that works in freestanding environments.

**How this differs from WASI guests:**

Some WebAssembly projects target `wasm32-wasip1` (WASI Preview 1) or `wasm32-wasip2` (WASI Preview 2). Those targets provide OS-like capabilities through WASI imports, so guests can use `println!` and other `std` features. This project deliberately avoids WASI to match the embedded architecture, where no OS exists. Instead, guests return strings through WIT exports, and the host decides what to do with them (print to terminal, send over UART, etc.).

---

## What Is AOT Precompilation

AOT stands for Ahead-Of-Time. It means compiling code before it is needed, rather than at runtime.

**The compilation pipeline in this project:**

1. **Rust source code** (`guest1/src/lib.rs`) is compiled by `rustc` to a **core WebAssembly module** (`guest1.wasm`). This target is `wasm32-unknown-unknown`.
2. The core module is wrapped by `ComponentEncoder` into a **WebAssembly component** (adds Component Model type metadata and the canonical ABI layer).
3. The component is precompiled by `engine.precompile_component()` into **Pulley bytecode** (`guest1.cwasm`). This is the AOT step.
4. The precompiled bytecode is embedded into the host binary via `include_bytes!`.

**Why AOT matters:**

Without AOT, the host would need to compile the WebAssembly component to Pulley bytecode every time it starts. This compilation takes time and requires the full Wasmtime compiler in the binary. With AOT, the compilation happens once (at build time on your laptop), and the result is a blob of Pulley bytecode that the host simply deserializes. No compiler needed at runtime.

This is critical for embedded systems: the RP2350 microcontroller does not have enough memory or processing power to run the Wasmtime compiler. It can only deserialize precompiled Pulley bytecode. By doing AOT compilation at build time, the embedded host can be small and fast.

**In this project:**

The `build.rs` script performs AOT compilation. The host uses `unsafe { Component::deserialize(engine, bytes) }` to load the precompiled bytecode. The `unsafe` is required because Wasmtime trusts that the bytes are valid precompiled output (not arbitrary data). This trust is upheld by the build script, which produced those bytes using Wasmtime itself.

---

## Project Structure

```
0x01_hello-world/
    Cargo.toml              # Host package manifest (wasmtime dependency + build-deps)
    build.rs                # AOT pipeline: compile guests, encode, precompile to Pulley
    host.rs                 # Host program: deserializes and runs guest components
    tests/
        integration.rs      # Integration tests for both guest components
    guest1/
        Cargo.toml          # Guest1 package manifest (wit-bindgen + dlmalloc)
        wit/
            world.wit       # WIT contract: export run: func() -> string
        src/
            lib.rs          # Guest1 implementation: #![no_std], returns string
    guest2/
        Cargo.toml          # Guest2 package manifest (wit-bindgen + dlmalloc)
        wit/
            world.wit       # WIT contract: export run(name), export describe
        src/
            lib.rs          # Guest2 implementation: #![no_std], greeting + describe
```

There are three separate Rust packages here:

1. **The host** (`Cargo.toml` + `host.rs` + `build.rs`): A normal Rust binary that depends on `wasmtime`. The build script compiles the guests and precompiles them. The binary deserializes and runs them.
2. **guest1** (`guest1/Cargo.toml` + `guest1/src/lib.rs`): A `#![no_std]` Rust library compiled to `wasm32-unknown-unknown`. Uses `wit-bindgen` and `dlmalloc`.
3. **guest2** (`guest2/Cargo.toml` + `guest2/src/lib.rs`): Same idea, different WIT contract and implementation.

---

## The WIT Files — Line by Line

### guest1/wit/world.wit

```wit
package component:guest1;
```

This declares the WIT package name. The format is `namespace:name`. Here, `component` is the namespace and `guest1` is the name. This identifier is used by tooling to match things together. It does not affect runtime behavior.

```wit
world guest1-world {
```

A `world` defines the complete set of imports and exports for a component. The name `guest1-world` is what the `wit_bindgen::generate!` macro uses to find the right world definition in the WIT file.

```wit
    export run: func() -> string;
```

This line says: "Any component targeting this world must export a function called `run` that takes no arguments and returns a string." The host will look for this function by name and verify the signature.

```wit
}
```

End of the world definition.

### guest2/wit/world.wit

```wit
package component:guest2;

world guest2-world {
    export run: func(name: option<string>) -> string;
    export describe: func() -> string;
}
```

This is different from guest1 in two ways:

1. `run` takes a parameter: `name: option<string>`. In WIT, `option<string>` means the value can either be a string or nothing (null/None). The Component Model handles encoding this across the host-guest boundary. It also returns a `string`.
2. There is a second export: `describe: func() -> string`. This function takes nothing and returns a string.

These two WIT files are why guest1 and guest2 behave differently. The WIT file is the contract. The Rust code implements the contract.

---

## Guest1 — Line by Line

File: `guest1/src/lib.rs`

```rust
#![no_std]
```

This attribute tells the Rust compiler: do not link the standard library. The guest runs on `wasm32-unknown-unknown` where there is no operating system, so `std` features like `println!`, file I/O, and threads are unavailable.

```rust
extern crate alloc;
```

This brings in the `alloc` crate, which provides heap-allocated types (`String`, `Vec`, `Box`, etc.) without requiring `std`. The `alloc` crate works with any global allocator — it does not depend on an OS.

```rust
use alloc::string::String;
```

Imports the `String` type from `alloc`. In `std` Rust, `String` comes from the standard library's prelude. In `no_std`, you import it explicitly from `alloc`.

```rust
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
```

This sets `dlmalloc` as the global memory allocator. The `#[global_allocator]` attribute tells Rust: "Use this allocator for all heap allocations." `dlmalloc` is a simple, portable allocator that works in freestanding environments like WebAssembly. Without this, the guest cannot allocate `String` or any other heap type. It is also required by the canonical ABI's `cabi_realloc` function, which the Component Model uses to allocate memory for passing data (like strings) across the host-guest boundary.

```rust
wit_bindgen::generate!({
    world: "guest1-world",
    path: "wit",
});
```

This macro reads `guest1/wit/world.wit`, finds the `guest1-world` world, and generates Rust code (traits, types, and glue) that matches the WIT contract. It creates the `Guest` trait with the `run()` method and the `export!` macro used below. This replaces the old `cargo-component` + `bindings.rs` approach — everything is generated inline at compile time.

```rust
struct Component;
```

This is a zero-sized struct. It exists only so you have a type to implement the `Guest` trait on. It holds no data.

```rust
export!(Component);
```

This macro call registers `Component` as the implementation that the Component Model runtime should use when the host calls exported functions. Without this line, the component would have no implementation backing the WIT exports, and instantiation would fail.

```rust
impl Guest for Component {
    fn run() -> String {
        String::from("guest1 run() called")
    }
}
```

This implements the `Guest` trait. When the host calls the `run` export, this function executes. It returns a `String` which the canonical ABI encodes and passes back to the host. No `println!`, no WASI — just a return value.

---

## Guest2 — Line by Line

File: `guest2/src/lib.rs`

The structure is identical to guest1, with these differences:

```rust
use alloc::format;
use alloc::string::String;
```

Guest2 also imports `format!` from `alloc` because its `run()` method uses string formatting.

```rust
const DEFAULT_NAME: &str = "world";
```

A constant string used as a fallback when the host passes `None` for the `name` parameter.

```rust
impl Guest for Component {
    fn run(name: Option<String>) -> String {
        let name = name.as_deref().unwrap_or(DEFAULT_NAME);
        format!("guest2 run() called: hello, {name}!")
    }
```

The `run` method takes `name: Option<String>` because the WIT file declares `run: func(name: option<string>) -> string`. The generated `Guest` trait requires this exact signature. `as_deref()` converts `Option<String>` to `Option<&str>`, and `unwrap_or(DEFAULT_NAME)` provides `"world"` when the value is `None`. The function returns a formatted greeting string.

```rust
    fn describe() -> String {
        String::from("guest2 has an extra `describe` export")
    }
}
```

The second export. The host looks for this function by name. If found, it calls it and prints the returned string. Guest1 does not have this, so the host does not try to call it for guest1.

---

## The Build Script — Line by Line

File: `build.rs`

The build script runs at `cargo build` time, before compiling `host.rs`. It compiles both guest crates, encodes them as WebAssembly components, AOT-precompiles them to Pulley bytecode, and writes the results to `OUT_DIR` for the host to embed.

### Constants

```rust
const GUEST1_CORE_WASM_PATH: &str = "guest1/target/wasm32-unknown-unknown/release/guest1.wasm";
const GUEST2_CORE_WASM_PATH: &str = "guest2/target/wasm32-unknown-unknown/release/guest2.wasm";
```

Paths to the core WebAssembly modules produced by compiling each guest crate.

```rust
const GUEST1_SERIALIZED_NAME: &str = "guest1.cwasm";
const GUEST2_SERIALIZED_NAME: &str = "guest2.cwasm";
```

Filenames for the AOT-precompiled Pulley bytecode artifacts written to `OUT_DIR`.

```rust
const GUEST1_COMPONENT_NAME: &str = "guest1.component.wasm";
const GUEST2_COMPONENT_NAME: &str = "guest2.component.wasm";
```

Filenames for the encoded component bytes (before AOT precompilation), also written to `OUT_DIR`. These are used by integration tests, which load components with `Component::new()` instead of `Component::deserialize()`.

### The compilation pipeline

```rust
fn compile_guest_wasm(guest_dir: &str) {
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(guest_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .expect("failed to build guest wasm crate");
    assert!(status.success(), "guest wasm build failed: {}", guest_dir);
}
```

Runs `cargo build` inside the guest directory targeting `wasm32-unknown-unknown`. The `--release` flag produces optimized output. `.env_remove("CARGO_ENCODED_RUSTFLAGS")` prevents the host's Rust flags from leaking into the guest build (they target different architectures).

```rust
fn pulley_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target("pulley64").expect("set pulley64 target");
    Engine::new(&config).expect("create pulley engine")
}
```

Creates a Wasmtime Engine configured identically to the runtime Engine in `host.rs`. This is critical: the precompiled bytecode is only valid if the runtime Engine has the same configuration as the build-time Engine. Any mismatch (different target, different features) causes `Component::deserialize` to fail.

```rust
fn encode_component(wasm_bytes: &[u8]) -> Vec<u8> {
    ComponentEncoder::default()
        .module(wasm_bytes)
        .expect("set core wasm module")
        .validate(true)
        .encode()
        .expect("encode component")
}
```

Takes raw core WebAssembly bytes and wraps them as a Component Model component. The `ComponentEncoder` reads the `wit-bindgen` metadata embedded in the core module, adds the canonical ABI layer, and produces a valid component binary. `.validate(true)` runs validation to catch encoding errors early.

```rust
fn precompile_component(engine: &Engine, component_bytes: &[u8]) -> Vec<u8> {
    engine
        .precompile_component(component_bytes)
        .expect("precompile component for pulley64")
}
```

The AOT step. Takes encoded component bytes and compiles them to Pulley bytecode. The output is a serialized blob that can be deserialized later with `Component::deserialize`. This is what makes runtime startup fast — no compilation needed.

```rust
fn compile_guest_to_pulley(...) {
    compile_guest_wasm(guest_dir);
    let core_wasm = read_core_wasm(core_wasm_path);
    let component = encode_component(&core_wasm);
    write_precompiled_component(out_dir, component_name, &component);
    let serialized = precompile_component(engine, &component);
    write_precompiled_component(out_dir, output_name, &serialized);
}
```

The full pipeline for one guest: compile, read, encode, write component (for tests), precompile, write serialized (for host). This function is called once for each guest.

### Rerun triggers

```rust
fn print_rerun_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=guest1/wit/world.wit");
    println!("cargo:rerun-if-changed=guest1/src/lib.rs");
    // ...
}
```

Tells Cargo to re-run the build script only if these specific files change. Without these lines, the build script would run on every `cargo build`, recompiling both guests even if nothing changed.

---

## The Host — Line by Line

File: `host.rs`

This is the heart of the project. Every line is explained below.

### Imports

```rust
use wasmtime::component::{Component, Linker};
```

- `Component`: Represents a compiled or deserialized WebAssembly component.
- `Linker`: Connects a component's imports (if any) to implementations the host provides. Our `#![no_std]` guests have no imports, so the Linker is empty.

```rust
use wasmtime::{Config, Engine, Result, Store};
```

- `Config`: A configuration builder for the Engine. You set options like target architecture and feature flags.
- `Engine`: The compiled-code cache and configuration holder. Deserialization requires an Engine with the same configuration used during precompilation.
- `Result`: Wasmtime's result type (`Result<T, wasmtime::Error>`).
- `Store`: The runtime state container. Holds your host state, WebAssembly memory, and instance data.

### Constants

```rust
const GUEST1_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest1.cwasm"));
const GUEST2_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest2.cwasm"));
```

These embed the precompiled Pulley bytecode directly into the host binary at compile time. `include_bytes!` reads the file that `build.rs` wrote to `OUT_DIR` and includes its contents as a `&[u8]` byte slice. At runtime, there is no file I/O — the bytes are already in memory.

```rust
const DEFAULT_GUEST_NAME: &str = "Pulley";
```

The default value passed to guest2's `run` when no CLI argument is provided.

### Functions

```rust
fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target("pulley64")?;
    Engine::new(&config)
}
```

Creates a Wasmtime Engine at runtime. The configuration **must match** what `build.rs` used during precompilation. If you change the target here but not in `build.rs` (or vice versa), `Component::deserialize` will fail.

```rust
fn load_component(engine: &Engine, bytes: &[u8]) -> Result<Component> {
    unsafe { Component::deserialize(engine, bytes) }
}
```

Deserializes precompiled Pulley bytecode into a `Component`. The `unsafe` is required because Wasmtime trusts that the bytes are valid precompiled output. If you passed arbitrary bytes, Wasmtime could exhibit undefined behavior. This invariant is upheld by `build.rs`, which produced these bytes using Wasmtime's own `precompile_component`.

This is the key difference from the older architecture: instead of `Component::from_file()` (which reads a `.wasm` file and compiles it at runtime), we use `Component::deserialize()` (which loads already-compiled bytecode). No compilation happens at runtime.

```rust
fn run_guest1(engine: &Engine, component: &Component) -> Result<String> {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let run = instance.get_typed_func::<(), (String,)>(&mut store, "run")?;
    let (result,) = run.call(&mut store, ())?;
    Ok(result)
}
```

Creates an empty Linker (our guests have no imports), creates a Store with `()` as the host state (no WASI context needed), instantiates the component, looks up the `run` export, calls it, and returns the result string.

Note the type parameter `(String,)` — that is a one-element tuple. The Component Model returns values as tuples. The `run` function returns one `string`, so the Rust side receives `(String,)`.

```rust
fn run_guest2(engine: &Engine, component: &Component, name: &str) -> Result<(String, String)> {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let run = instance.get_typed_func::<(Option<String>,), (String,)>(&mut store, "run")?;
    let (run_result,) = run.call(&mut store, (Some(name.to_string()),))?;
    let describe = instance.get_typed_func::<(), (String,)>(&mut store, "describe")?;
    let (desc_result,) = describe.call(&mut store, ())?;
    Ok((run_result, desc_result))
}
```

Same pattern but for guest2, which has two exports. `run` takes `(Option<String>,)` as input and returns `(String,)`. `describe` takes no input and returns `(String,)`.

```rust
fn parse_name() -> String {
    std::env::args().nth(1).unwrap_or_else(|| DEFAULT_GUEST_NAME.to_string())
}
```

Reads the first CLI argument. If none is provided, defaults to `"Pulley"`.

```rust
fn run() -> Result<()> {
    let name = parse_name();
    println!("Building Pulley component engine...");
    let engine = build_engine()?;
    // ... deserialize guests, call exports, print results
    Ok(())
}
```

The main logic function. Called by `main()`.

```rust
fn main() -> Result<()> {
    run()
}
```

The program entry point. Returns `wasmtime::Result<()>` so errors are printed automatically.

---

## The Engine — In Depth

The `Engine` is the most foundational object in Wasmtime. It is created once and reused for every component you load.

### What the Engine does

1. **Holds configuration**: The Engine preserves the `Config` settings (target architecture, feature flags) and enforces them on every operation.

2. **Validates precompiled code**: When you call `Component::deserialize`, the Engine checks that the precompiled bytes were produced with a compatible configuration.

3. **Caches compiled code**: If you load the same component twice with the same Engine, the compilation result can be reused.

### The configuration lines

**`config.wasm_component_model(true)`** — Enables Component Model support. Without this, Wasmtime can only load core WebAssembly modules (the old, number-only kind). With this enabled, Wasmtime understands components, WIT types, and the canonical ABI.

**`config.target("pulley64")?`** — Sets the target to `pulley64`. This tells the Engine that all components were compiled (or should be compiled) for the Pulley64 bytecode format. This must match what `build.rs` used.

**`Engine::new(&config)`** — Creates the Engine from the configuration.

### Why the build-time and runtime engines must match

`Component::deserialize` does not re-compile anything. It takes bytes that were precompiled by `engine.precompile_component()`. If the runtime Engine has a different configuration than the build-time Engine, the precompiled bytes are invalid. Wasmtime checks this and returns an error.

---

## The Linker — In Depth

```rust
let linker = Linker::<()>::new(engine);
```

The `Linker` connects a component's imports to host-side implementations.

### What imports are

When a WebAssembly component is compiled, it may contain import declarations. These are functions the component needs but does not implement itself. It expects the host to provide them.

### Our guests have no imports

Because our guests use `#![no_std]` and target `wasm32-unknown-unknown`, they have no WASI imports. They do not call `println!`, do not access files, and do not need any host-provided functions. They only export functions and return values through the canonical ABI.

This means the Linker is empty — `Linker::<()>::new(engine)` creates a Linker with nothing registered, and that is sufficient. The type parameter `()` means the Store holds no host state (just the unit type).

### When you would need a non-empty Linker

If you added WIT imports to a guest's `world.wit` (for example, `import log: func(msg: string);`), you would need to register that function in the Linker before instantiation. The Linker maps import names to Rust closures or functions that implement them.

---

## The Store — In Depth

```rust
let mut store = Store::new(engine, ());
```

The `Store` is the runtime state container.

### What the Store holds

1. **Your host state** (`()`): The type parameter. In this project, it is the unit type because we have no WASI context to store.
2. **WebAssembly linear memory**: When a guest allocates memory (via `dlmalloc`), it lives inside the Store.
3. **Global variables**: WebAssembly global values are stored here.
4. **Function handles**: Typed function references retrieved from instances live in the Store.
5. **Instance data**: After instantiation, the running instance's state lives in the Store.

### Why Store<()> instead of a custom struct

In the older WASI-based architecture, the Store held a `HostState` struct containing `WasiCtx` and `ResourceTable`. Since our guests no longer use WASI, there is nothing for the host to provide. The unit type `()` is the simplest possible state — it holds no data.

### A new Store per component

Each guest component gets its own fresh Store. This means each component has its own isolated memory space. One guest cannot interfere with another guest's memory or state.

---

## The Component — In Depth

```rust
unsafe { Component::deserialize(engine, bytes) }
```

A `Component` represents a compiled WebAssembly component. It contains the compiled Pulley bytecode and type metadata (what it imports and exports), ready to be instantiated.

### Component::deserialize vs Component::new vs Component::from_file

There are several ways to create a Component:

- **`Component::from_file(engine, path)`** — Reads a `.wasm` file from disk and compiles it at runtime. Used in the integration tests (which compile components with `Component::new` from embedded bytes). Not used in the host binary.
- **`Component::new(engine, bytes)`** — Takes raw component bytes and compiles them at runtime. Used in the integration tests.
- **`Component::deserialize(engine, bytes)`** — Takes precompiled Pulley bytecode and loads it without any compilation. Used in the host binary. This is the fastest option and the only one suitable for embedded targets.

### Why deserialize is unsafe

`Component::deserialize` trusts that the bytes are valid precompiled Wasmtime output. If you pass random or malicious bytes, Wasmtime could execute invalid code, leading to undefined behavior. The `unsafe` annotation makes this trust explicit. In this project, the trust is justified because `build.rs` produced the bytes using `engine.precompile_component()`, and the bytes are embedded in the binary via `include_bytes!`.

---

## The Instance — In Depth

```rust
let instance = linker.instantiate(&mut store, &component)?;
```

An `Instance` is a running component. Instantiation connects a compiled Component to a Store and a Linker, resolving all imports.

### What instantiation does

1. **Resolves imports**: If the component has imports, the Linker provides them. Our guests have no imports, so this step is a no-op.
2. **Allocates memory**: The component's linear memory is allocated inside the Store.
3. **Runs start functions**: If the component has initialization code, it runs now.
4. **Returns the Instance**: An object you can use to look up and call exported functions.

### Calling exports

After instantiation, you call exports through the Instance:

```rust
let run = instance.get_typed_func::<(), (String,)>(&mut store, "run")?;
let (result,) = run.call(&mut store, ())?;
```

**`get_typed_func::<(), (String,)>(&mut store, "run")`** — Look up an exported function named `"run"` and verify that it takes no parameters (`()`) and returns one string (`(String,)`). The type parameters are checked at this point. If the function's actual signature does not match, this call returns an error.

**`run.call(&mut store, ())`** — Execute the function. The `&mut store` is needed because the function may modify memory or trap. The result is `(String,)` — a one-element tuple containing the returned string.

For guest2, the signature is different:

```rust
let run = instance.get_typed_func::<(Option<String>,), (String,)>(&mut store, "run")?;
let (result,) = run.call(&mut store, (Some(name.to_string()),))?;
```

`(Option<String>,)` is a one-element tuple for input. The Component Model's canonical ABI handles encoding the Rust `Option<String>` into the WebAssembly linear memory format that the guest expects. The return type `(String,)` is decoded automatically. You write normal Rust types and the runtime handles the conversion.

---

## How They All Connect

Here is the dependency chain, top to bottom:

**Build time (build.rs):**

```
Guest Rust source (lib.rs)
  |
  v
cargo build --target wasm32-unknown-unknown -> core .wasm module
  |
  v
ComponentEncoder -> component .wasm (with WIT metadata)
  |
  v
Engine (pulley64) -> engine.precompile_component() -> .cwasm (Pulley bytecode)
  |
  v
include_bytes! embeds .cwasm into host binary
```

**Runtime (host.rs):**

```
Config
  |
  v
Engine (holds configuration, validates precompiled code)
  |
  +---> Component::deserialize(engine, bytes) -> Component
  |
  +---> Linker<()>::new(engine) -> empty Linker (no imports needed)
  |
  +---> Store::new(engine, ()) -> Store with unit state
          |
          +---> linker.instantiate(&mut store, &component) -> Instance
                  |
                  +---> instance.get_typed_func("run") -> TypedFunc
                          |
                          +---> run.call(&mut store, args) -> String
                                  (Pulley interprets guest bytecode)
```

The Engine is the root of everything. You cannot create a Store, Linker, or Component without one. The Engine's configuration must match between build time and runtime.

---

## The Integration Tests — Line by Line

File: `tests/integration.rs`

The integration tests verify that the guest components work correctly without running the full host binary. They use the same Wasmtime APIs as `host.rs`, but load components via `Component::new()` (runtime compilation) instead of `Component::deserialize()` (AOT precompiled).

### Test infrastructure

The tests define helper functions:

- **`create_engine()`** — Creates a default Engine (no Pulley target, since tests compile at runtime).
- **`compile_component(engine, wasm)`** — Compiles raw component bytes into a Component via `Component::new`.
- **`call_run_no_args(engine, component)`** — Instantiates and calls `run()` returning a String.
- **`call_run_with_name(engine, component, name)`** — Instantiates and calls `run(Option<String>)`.
- **`call_describe(engine, component)`** — Instantiates and calls `describe()`.

### Component bytes

```rust
const GUEST1_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest1.component.wasm"));
const GUEST2_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest2.component.wasm"));
```

The tests use the encoded component bytes (not the AOT-precompiled `.cwasm` files). These are the same bytes that `build.rs` wrote before the precompilation step. `Component::new()` compiles them at test time using the default engine configuration (native code, not Pulley), which is fine for testing on your laptop.

### What each test verifies

| Test                                           | What it checks                                                       |
| ---------------------------------------------- | -------------------------------------------------------------------- |
| `test_guest1_component_loads`                  | guest1 component bytes are valid                                     |
| `test_guest2_component_loads`                  | guest2 component bytes are valid                                     |
| `test_guest1_exports_run_function`             | guest1 exports `run` with signature `() -> (String,)`                |
| `test_guest2_exports_run_function`             | guest2 exports `run` with signature `(Option<String>,) -> (String,)` |
| `test_guest2_exports_describe_function`        | guest2 exports `describe` with signature `() -> (String,)`           |
| `test_guest1_does_not_export_describe`         | guest1 does NOT have a `describe` export                             |
| `test_guest1_run_returns_expected_string`      | `run()` returns a string containing "guest1"                         |
| `test_guest2_run_returns_expected_string`      | `run(Some("Pulley"))` returns a string containing "guest2"           |
| `test_guest2_describe_returns_expected_string` | `describe()` returns the exact expected string                       |
| `test_guest1_has_no_wasi_imports`              | guest1 has no WASI-related imports                                   |
| `test_guest2_has_no_wasi_imports`              | guest2 has no WASI-related imports                                   |
| `test_guest2_run_default_name`                 | `run(None)` returns a string containing "world"                      |
| `test_guest2_run_custom_name`                  | `run(Some("Pulley"))` returns a string containing "Pulley"           |
| `test_guest1_run_exact_message`                | `run()` returns exactly "guest1 run() called"                        |
| `test_guest2_run_exact_greeting`               | `run(Some("Pulley"))` returns exactly the expected greeting          |

### Why tests use Component::new instead of Component::deserialize

The AOT-precompiled `.cwasm` files target `pulley64`. Test code runs natively on your laptop (x86-64 or AArch64). To execute guest code natively in tests, you need the default engine (which compiles to native machine code), not a Pulley-targeting engine. The encoded component bytes (`.component.wasm`) work with any engine configuration since `Component::new()` compiles them at runtime.

---

## How to Build and Run

### Prerequisites

Install Rust with the `wasm32-unknown-unknown` target:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

### Build everything

```
cargo build
```

This single command:

1. Runs `build.rs`, which compiles both guest crates, encodes them as components, and AOT-precompiles them to Pulley bytecode.
2. Compiles `host.rs` with the precompiled guest artifacts embedded via `include_bytes!`.

No separate guest build step. No `cargo-component`. Just `cargo build`.

### Run the host (default name)

```
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

### Run the host with a custom name

```
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

The `--` separates `cargo` arguments from your program's arguments. `"Kevin"` becomes `args[1]` in host.rs.

### Run tests

```
cargo test
```

This compiles the test binary and runs all 15 integration tests.

---

## Why This Matters for Embedded Systems

### The problem

Embedded microcontrollers (RP2350, ESP32, STM32, etc.) run bare-metal — no operating system, no process isolation, no memory protection. Every piece of firmware has full access to all hardware. If you load untrusted code, it can overwrite memory, corrupt peripherals, or brick the device.

### The WebAssembly solution

WebAssembly runs in a sandbox. A guest component:

- Cannot access hardware registers unless the host explicitly exposes them through WIT interfaces.
- Cannot read or write memory outside its own linear memory.
- Cannot call any function unless the host links it in through the Linker.
- Can be updated independently of the host firmware by replacing the `.wasm` file.

### How Pulley enables it

Microcontrollers like the RP2350 (Cortex-M33, ARMv8-M) use processor architectures that Cranelift's native code generator does not support. Pulley solves this by providing a portable interpreter that runs on any processor. The host firmware includes the Pulley interpreter, and guest components are precompiled to Pulley bytecode at build time.

### What the embedded version looks like

The [embedded-wasm-uart-rp2350](https://github.com/mytechnotalent/embedded-wasm-uart-rp2350) project uses the exact same architecture as this tutorial:

| This tutorial (laptop)                        | Embedded version (RP2350)                                   |
| --------------------------------------------- | ----------------------------------------------------------- |
| `host.rs` runs on your Mac/Linux/Windows      | `src/main.rs` runs bare-metal on the RP2350                 |
| Config targets `pulley64`                     | Config targets `pulley32`                                   |
| `build.rs` precompiles to Pulley bytecode     | `build.rs` precompiles to Pulley bytecode                   |
| `Component::deserialize(include_bytes!(...))` | `Component::deserialize(include_bytes!(...))`               |
| Guests use `#![no_std]` + `dlmalloc`          | Guests use `#![no_std]` + `dlmalloc`                        |
| Guests return strings via WIT exports         | Guests call host-provided WIT imports (UART `write_byte()`) |
| `Linker<()>` — empty, no imports              | `Linker<Host>` — registers UART hardware functions          |
| `Store<()>` — no host state                   | `Store<Host>` — holds UART peripheral state                 |

The mental model is identical. The host creates an Engine, deserializes precompiled Components, builds a Linker, creates a Store, instantiates, and calls exports. The only differences are the target (`pulley32` vs `pulley64`) and the imports (UART hardware vs nothing in this tutorial).

### The key insight

This tutorial project teaches you the architecture. Once you understand Engine, Store, Linker, Component, and Instance here on your laptop, you understand exactly how it works on an embedded microcontroller. The concepts are the same. The only thing that changes is what hardware the host provides to the guest through WIT interfaces.

---

## Glossary

| Term                         | Definition                                                                                                                        |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **WebAssembly (Wasm)**       | A portable binary instruction format that runs inside a sandboxed virtual machine.                                                |
| **Wasmtime**                 | A production WebAssembly runtime written in Rust by the Bytecode Alliance.                                                        |
| **Pulley**                   | A portable interpreter inside Wasmtime that executes Pulley bytecode on any CPU, including embedded microcontrollers.             |
| **WIT**                      | WebAssembly Interface Types. A language for defining function signatures between host and guest.                                  |
| **Component Model**          | A standard on top of Wasm that adds rich types (strings, records, options) and WIT-based interfaces.                              |
| **`#![no_std]`**             | Rust attribute that disables the standard library. Required for `wasm32-unknown-unknown` targets without OS support.              |
| **`dlmalloc`**               | A portable memory allocator used as the global allocator in `no_std` guests. Required for `String`, `Vec`, and the canonical ABI. |
| **`wit-bindgen`**            | A Rust crate whose `generate!` macro produces Rust traits and types from WIT definitions at compile time.                         |
| **AOT**                      | Ahead-Of-Time compilation. Compiling WebAssembly to Pulley bytecode at build time so no compiler is needed at runtime.            |
| **`ComponentEncoder`**       | A type from `wit-component` that wraps a core WebAssembly module as a Component Model component.                                  |
| **Engine**                   | The Wasmtime object that holds configuration and validates precompiled code. Created once, reused for all components.             |
| **Config**                   | Builder for Engine settings: target architecture, feature flags, memory limits.                                                   |
| **Store**                    | The runtime state container. Holds your host state, WebAssembly memory, globals, and instance data. One per component instance.   |
| **Linker**                   | A registry of host-provided functions. Maps import names to implementations. Empty when guests have no imports.                   |
| **Component**                | A compiled or deserialized WebAssembly component. Contains code and type metadata but is not yet running.                         |
| **Instance**                 | A running component. Created by `linker.instantiate()`. You call exported functions through the Instance.                         |
| **`get_typed_func`**         | Looks up an exported function by name and verifies its parameter/return types at lookup time.                                     |
| **`call`**                   | Executes a typed function. The Pulley interpreter runs the guest code.                                                            |
| **`include_bytes!`**         | Rust macro that embeds a file's bytes into the binary at compile time.                                                            |
| **`Component::deserialize`** | Loads precompiled Pulley bytecode without any runtime compilation. The fastest way to load a component.                           |
| **Canonical ABI**            | The encoding format the Component Model uses to convert high-level types (strings, options, records) into bytes in linear memory. |
| **Linear memory**            | A contiguous byte array inside the Store that the guest uses for heap allocations. Isolated from host memory.                     |
| **Cranelift**                | Wasmtime's native code compiler. Generates machine code for x86-64, AArch64, etc. Does not support embedded Cortex-M.             |
| **`cabi_realloc`**           | A function the canonical ABI requires guests to export for memory allocation. Provided by `dlmalloc`.                             |
