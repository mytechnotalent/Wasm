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
8. [What Is WASI](#what-is-wasi)
9. [Project Structure](#project-structure)
10. [The WIT Files — Line by Line](#the-wit-files--line-by-line)
11. [Guest1 — Line by Line](#guest1--line-by-line)
12. [Guest2 — Line by Line](#guest2--line-by-line)
13. [The Host — Line by Line](#the-host--line-by-line)
14. [The HostState Struct — In Depth](#the-hoststate-struct--in-depth)
15. [The Engine — In Depth](#the-engine--in-depth)
16. [The Linker — In Depth](#the-linker--in-depth)
17. [The Store — In Depth](#the-store--in-depth)
18. [The Component — In Depth](#the-component--in-depth)
19. [The Instance — In Depth](#the-instance--in-depth)
20. [How They All Connect](#how-they-all-connect)
21. [The Integration Tests — Line by Line](#the-integration-tests--line-by-line)
22. [How to Build and Run](#how-to-build-and-run)
23. [Why This Matters for Embedded Systems](#why-this-matters-for-embedded-systems)
24. [Glossary](#glossary)

---

## The Big Picture

This project has two guest programs and one host program. The guests are written in Rust, compiled to WebAssembly, and packaged as "components." The host is also written in Rust. The host loads those guest components at runtime, connects them to the outside world (so they can print to the terminal), and then calls functions that the guests export.

The entire execution flow looks like this:

```
You type: cargo run --bin hello

    main() starts in host.rs
        |
        v
    build_engine() creates a Wasmtime Engine configured for Pulley
        |
        v
    For each guest component path in COMPONENT_PATHS:
        |
        v
    Component::from_file() reads the .wasm file from disk
        |
        v
    build_linker() creates a Linker that provides WASI imports
        |
        v
    build_store() creates a Store holding HostState (WasiCtx + ResourceTable)
        |
        v
    linker.instantiate() connects the component's imports to the linker's exports
        |
        v
    instance.get_typed_func() looks up the "run" export by name
        |
        v
    run.call() executes the guest function through Pulley
        |
        v
    Guest prints to stdout via WASI -> text appears in your terminal
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
3. Provide any imports the guest code needs (like printing to a screen).
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

We set `config.target("pulley64")` in the host. This tells Wasmtime: "Do not compile to native machine code. Compile to Pulley bytecode instead." We use `pulley64` because we are running on a 64-bit system (your Mac). On a 32-bit embedded target like the RP2350, you would use `pulley32`.

This project runs on your laptop to teach you the concepts. The same architecture — host loads guest component, executes via Pulley — is exactly what runs on an embedded microcontroller like the RP2350 in the [embedded-wasm-uart-rp2350](https://github.com/mytechnotalent/embedded-wasm-uart-rp2350) project.

---

## What Is WIT

WIT stands for WebAssembly Interface Types. It is a plain-text language for defining function signatures that cross the boundary between a host and a guest.

**Why it exists:**

When the host wants to call a function inside a guest, both sides need to agree on the function's name, what parameters it takes, and what it returns. WIT is how you write that agreement down.

Think of WIT as a contract. The guest says: "I promise to export a function called `run` that takes no arguments." The host says: "I promise to look for a function called `run` that takes no arguments." If both sides follow the contract, everything works. If they disagree (the guest exports `run` with a parameter but the host expects no parameter), instantiation fails.

**What a WIT file looks like:**

```wit
package component:guest1;

world example {
    export run: func();
}
```

This says:

- `package component:guest1;` — This WIT file belongs to a package called `component:guest1`. The package name is just an identifier.
- `world example` — A "world" is a complete description of what a component imports and exports. The name `example` is arbitrary.
- `export run: func();` — The component promises to export a function called `run` that takes no parameters and returns nothing.

---

## What Is the Component Model

The Component Model is a standard that sits on top of WebAssembly. Plain ("core") WebAssembly only understands numbers — integers and floats. It has no concept of strings, lists, records, or other high-level types. It also has no standard way to describe imports and exports beyond raw function signatures with numeric parameters.

The Component Model adds:

1. **Rich types**: Strings, lists, records, options, results, enums, flags, and more. When your guest function takes an `Option<String>`, the Component Model handles converting that Rust type into bytes the guest can receive.
2. **WIT-based interfaces**: Instead of raw numbered imports/exports, components use named interfaces defined in WIT files.
3. **Composition**: Multiple components can be wired together, with one component's exports satisfying another component's imports.

**In plain terms:**

Without the Component Model, you can only pass numbers between the host and guest. With the Component Model, you can pass strings, structs, options, and other complex types across the boundary, and the runtime handles all the memory layout and encoding automatically.

In this project, we enable the Component Model with `config.wasm_component_model(true)` in the host, and we use `Component::from_file` (not `Module::from_file`) to load guest artifacts. Everything flows through the Component Model.

---

## What Is WASI

WASI stands for WebAssembly System Interface. It provides standardized interfaces for things a program normally gets from an operating system:

- Reading and writing to stdout/stderr (printing)
- Reading environment variables
- Accessing the filesystem
- Getting the current time
- Generating random numbers

**Why it matters:**

A WebAssembly guest runs in a sandbox. It cannot print to the terminal by itself because it has no direct access to your operating system. When guest code calls `println!("hello")`, that `println!` macro eventually needs to write bytes to stdout. In a normal Rust program, the standard library calls the operating system directly. In a WebAssembly guest, there is no operating system. Instead, `println!` goes through WASI — the guest calls a WASI function to write bytes, and the host implements that WASI function by writing to the real stdout.

**In this project:**

The guests use `println!()`. For that to work:

1. The guest component automatically imports WASI interfaces (cargo-component handles this).
2. The host registers WASI implementations into the Linker with `wasmtime_wasi::p2::add_to_linker_sync(&mut linker)`.
3. The host configures the Store's `WasiCtx` to inherit the host's stdio with `WasiCtx::builder().inherit_stdio().build()`.

When guest code calls `println!()`, the call chain is: guest `println!` -> WASI `fd_write` import -> host's WASI implementation -> real stdout on your terminal.

---

## Project Structure

```
0x01_hello-world/
    Cargo.toml              # Host package manifest (wasmtime + wasmtime-wasi dependencies)
    host.rs                 # Host program: loads and runs guest components
    tests/
        integration.rs      # Integration tests for both guest components
    guest1/
        Cargo.toml          # Guest1 package manifest (wit-bindgen-rt dependency)
        wit/
            world.wit       # WIT contract: exports run: func()
        src/
            lib.rs          # Guest1 implementation: prints "guest1 run() called"
            bindings.rs     # Auto-generated by cargo-component (do not edit)
    guest2/
        Cargo.toml          # Guest2 package manifest (wit-bindgen-rt dependency)
        wit/
            world.wit       # WIT contract: exports run: func(name: option<string>)
                            #               exports describe: func() -> string
        src/
            lib.rs          # Guest2 implementation: greeting with optional name + describe
            bindings.rs     # Auto-generated by cargo-component (do not edit)
```

There are three separate Rust packages here:

1. **The host** (`Cargo.toml` + `host.rs`): A normal Rust binary that depends on `wasmtime` and `wasmtime-wasi`. It compiles and runs on your machine (or on an embedded target).
2. **guest1** (`guest1/Cargo.toml` + `guest1/src/lib.rs`): A Rust library compiled to WebAssembly by `cargo-component`. It produces `guest1.wasm`.
3. **guest2** (`guest2/Cargo.toml` + `guest2/src/lib.rs`): Same idea, different WIT contract and implementation. It produces `guest2.wasm`.

---

## The WIT Files — Line by Line

### guest1/wit/world.wit

```wit
package component:guest1;
```

This declares the WIT package name. The format is `namespace:name`. Here, `component` is the namespace and `guest1` is the name. This identifier is used by tooling to match things together. It does not affect runtime behavior.

```wit
world example {
```

A `world` defines the complete set of imports and exports for a component. The name `example` is just a label. You could call it anything. A component targets exactly one world.

```wit
    export run: func();
```

This line says: "Any component targeting this world must export a function called `run` that takes no arguments and returns nothing." The host will look for this function by name.

```wit
}
```

End of the world definition.

### guest2/wit/world.wit

```wit
package component:guest2;

world example {
    export run: func(name: option<string>);
    export describe: func() -> string;
}
```

This is different from guest1 in two ways:

1. `run` takes a parameter: `name: option<string>`. In WIT, `option<string>` means the value can either be a string or nothing (null/None). The Component Model handles encoding this across the host-guest boundary.
2. There is a second export: `describe: func() -> string`. This function takes nothing and returns a string.

These two WIT files are why guest1 and guest2 behave differently. The WIT file is the contract. The Rust code implements the contract.

---

## Guest1 — Line by Line

File: `guest1/src/lib.rs`

```rust
#[allow(warnings)]
mod bindings;
```

This declares a Rust module called `bindings` whose code lives in `guest1/src/bindings.rs`. That file is auto-generated by `cargo-component` every time you build. It reads `guest1/wit/world.wit` and generates Rust traits and types that match the WIT contract. You never edit `bindings.rs` by hand. The `#[allow(warnings)]` suppresses compiler warnings from the generated code.

```rust
use bindings::Guest;
```

This imports the `Guest` trait from the generated bindings. The `Guest` trait has one method: `fn run()`. This matches `export run: func();` from the WIT file. The code generator created this trait for you.

```rust
struct Component;
```

This is a zero-sized struct. It exists only so you have a type to implement the `Guest` trait on. It holds no data.

```rust
impl Guest for Component {
    fn run() {
        println!("guest1 run() called");
    }
}
```

This implements the `Guest` trait. When the host calls the `run` export, this function executes. `println!()` writes to stdout through WASI.

```rust
bindings::export!(Component with_types_in bindings);
```

This macro call registers `Component` as the implementation that the Component Model runtime should use when the host calls exported functions. Without this line, the component would have no implementation backing the WIT exports, and instantiation would fail.

---

## Guest2 — Line by Line

File: `guest2/src/lib.rs`

The structure is identical to guest1, with these differences:

```rust
const DEFAULT_NAME: &str = "world";
```

A constant string used as a fallback when the host passes `None` for the `name` parameter.

```rust
impl Guest for Component {
    fn run(name: Option<String>) {
        let name = name.as_deref().unwrap_or(DEFAULT_NAME);
        println!("guest2 run() called: hello, {name}!");
    }
```

The `run` method takes `name: Option<String>` because the WIT file declares `run: func(name: option<string>)`. The generated `Guest` trait requires this exact signature. `as_deref()` converts `Option<String>` to `Option<&str>`, and `unwrap_or(DEFAULT_NAME)` provides `"world"` when the value is `None`.

```rust
    fn describe() -> String {
        "guest2 has an extra `describe` export".to_string()
    }
}
```

The second export. The host looks for this function by name. If found, it calls it and prints the returned string. Guest1 does not have this, so the host skips it for guest1.

---

## The Host — Line by Line

File: `host.rs`

This is the heart of the project. Every line is explained below.

### Imports

```rust
use wasmtime::component::{Component, Linker, ResourceTable};
```

- `Component`: Represents a compiled WebAssembly component loaded from a `.wasm` file.
- `Linker`: Connects a component's imports (what it needs from the outside) to implementations the host provides.
- `ResourceTable`: A table that tracks WASI resources (file handles, sockets, etc.) by integer ID.

```rust
use wasmtime::{Config, Engine, Result, Store};
```

- `Config`: A configuration builder for the Engine. You set options like target architecture, memory limits, and feature flags.
- `Engine`: The compiled-code cache and configuration holder. All compilation happens through the Engine.
- `Result`: Wasmtime's result type (re-export of `anyhow::Result`).
- `Store`: The runtime state container. Holds your custom host state, WebAssembly memory, function handles, and instance data.

```rust
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
```

- `WasiCtx`: The WASI context. It holds configuration for stdin, stdout, stderr, filesystem access, environment variables, and other OS-like capabilities that the guest can use.
- `WasiCtxView`: A struct that bundles a mutable reference to `WasiCtx` and a mutable reference to `ResourceTable`. Wasmtime-wasi functions need both together.
- `WasiView`: A trait your host state must implement so that wasmtime-wasi knows how to find the `WasiCtx` and `ResourceTable` inside your custom state.

### Constants

```rust
const DEFAULT_GUEST_NAME: &str = "Pulley";
```

The default value passed to guest2's `run` when no CLI argument is provided.

```rust
const PULLEY_TARGET: &str = "pulley64";
```

The target string passed to `Config::target()`. This tells Wasmtime to compile WebAssembly to Pulley bytecode instead of native machine code. `pulley64` is for 64-bit systems. On a 32-bit embedded target you would use `pulley32`.

```rust
const COMPONENT_PATHS: [&str; 2] = [
    "guest1/target/wasm32-wasip1/debug/guest1.wasm",
    "guest2/target/wasm32-wasip1/debug/guest2.wasm",
];
```

Filesystem paths to the compiled guest component `.wasm` files. These files are produced by `cargo component build` for each guest package. The host loads them at runtime with `Component::from_file`.

---

## The HostState Struct — In Depth

```rust
struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
}
```

This struct is the host-side state that Wasmtime carries through every operation. Every `Store` holds exactly one instance of your host state type.

### Why does it exist?

When a guest calls a WASI function (like writing to stdout), Wasmtime needs to know: where is stdout configured? Is the guest allowed to write to files? What environment variables are available? All of these answers live inside `WasiCtx`.

Similarly, when a guest opens a file or creates a resource, Wasmtime tracks that resource by an integer handle in the `ResourceTable`. The guest never gets a raw pointer — it gets a small integer ID, and the `ResourceTable` maps that ID back to the actual resource on the host side.

### The two fields

**`ctx: WasiCtx`** — Built with `WasiCtx::builder().inherit_stdio().build()`. The `.inherit_stdio()` call means the guest's stdout/stderr are wired directly to the host process's stdout/stderr. When the guest calls `println!()`, the text appears in your terminal. Without `.inherit_stdio()`, the guest's output would go nowhere.

**`table: ResourceTable`** — Created empty with `ResourceTable::new()`. In this project, no guest opens files or creates resources, so the table stays empty. But it must exist because WASI requires it during linking.

### The WasiView trait implementation

```rust
impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}
```

Wasmtime-wasi does not know what your host state struct looks like. It could have 2 fields or 200. The `WasiView` trait is how you tell wasmtime-wasi: "Here is where the `WasiCtx` and `ResourceTable` live inside my struct."

The `ctx()` method returns a `WasiCtxView` which bundles mutable references to both fields. Wasmtime-wasi calls this method internally whenever a guest invokes a WASI import.

---

## The Engine — In Depth

```rust
fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target(PULLEY_TARGET)?;
    Engine::new(&config)
}
```

The `Engine` is the most foundational object in Wasmtime. It is created once and reused for every component you load.

### What the Engine does

1. **Holds the compiler**: The Engine contains the code-generation backend (Cranelift for native, or the Pulley bytecode emitter when targeting Pulley). When you call `Component::from_file`, the Engine compiles the WebAssembly bytecode.

2. **Caches compiled code**: If you load the same component twice with the same Engine, the compilation result can be reused.

3. **Enforces configuration**: Every Store, Linker, and Component created from this Engine must use the same configuration. You cannot mix an Engine configured for `pulley64` with a Component compiled by a different Engine configured for `x86_64`.

### The configuration lines

**`config.wasm_component_model(true)`** — Enables Component Model support. Without this, Wasmtime can only load core WebAssembly modules (the old, number-only kind). With this enabled, Wasmtime understands components, WIT types, and the canonical ABI (the encoding that converts high-level types like strings into bytes).

**`config.target(PULLEY_TARGET)?`** — Sets the compilation target to `pulley64`. Instead of generating native machine code, the Engine's compiler will emit Pulley bytecode. At runtime, the Pulley interpreter reads this bytecode and executes it instruction by instruction. This is what makes the code portable to embedded targets.

**`Engine::new(&config)`** — Creates the Engine from the configuration. This is an expensive operation (it initializes the compiler), which is why you do it once and reuse the Engine.

---

## The Linker — In Depth

```rust
fn build_linker(engine: &Engine) -> Result<Linker<HostState>> {
    let mut linker = Linker::<HostState>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    Ok(linker)
}
```

The `Linker` connects a component's imports to host-side implementations.

### What imports are

When a WebAssembly component is compiled, it may contain import declarations. These are functions the component needs but does not implement itself. It expects the host to provide them.

In this project, the guests use `println!()`, which internally calls WASI functions. Those WASI functions are imports. The guest `.wasm` file says: "I need a function called `wasi:cli/stdout` to be provided at instantiation time."

### What the Linker does

The Linker is a registry of host-provided functions. You register functions into the Linker, and when you instantiate a component, the Linker connects the component's imports to the registered functions.

**`Linker::<HostState>::new(engine)`** — Creates an empty Linker. The type parameter `HostState` tells the Linker what type of host state lives inside the Store. This matters because host-provided functions receive a mutable reference to the Store, and through it, to your `HostState`.

**`wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?`** — This single call registers ALL of the WASI Preview 2 interfaces into the Linker. This includes stdout, stderr, stdin, filesystem, clocks, random, environment variables, and more. The `_sync` suffix means these are synchronous (blocking) implementations, as opposed to async.

After this call, the Linker can satisfy any WASI import that a guest component requires.

### Why it takes `&Engine`

The Linker needs the Engine reference to ensure type-safety: the compiler settings, feature flags, and type encodings must match between the Linker and any Component it will be used with.

---

## The Store — In Depth

```rust
fn build_store(engine: &Engine) -> Store<HostState> {
    let state = HostState {
        ctx: WasiCtx::builder().inherit_stdio().build(),
        table: ResourceTable::new(),
    };
    Store::new(engine, state)
}
```

The `Store` is the runtime state container. It is the most-used object during execution.

### What the Store holds

1. **Your host state** (`HostState`): The custom struct you defined, accessible via `store.data()` and `store.data_mut()`.
2. **WebAssembly linear memory**: When a guest allocates memory, it lives inside the Store.
3. **Global variables**: WebAssembly global values are stored here.
4. **Function handles**: Typed function references retrieved from instances live in the Store.
5. **Instance data**: After instantiation, the running instance's state lives in the Store.

### Why a new Store per component

In this project, `build_store` is called inside `run_component`, which means each guest component gets its own fresh Store. This is intentional:

- Each component gets its own isolated memory space.
- Each component gets its own `WasiCtx` (its own stdout configuration).
- One guest cannot interfere with another guest's memory or state.

### The Store's type parameter

`Store<HostState>` means: "This Store carries a `HostState` value inside it." Every time Wasmtime needs to call a host function (like a WASI import), it passes `&mut Store<HostState>` to that function, which can then access `store.data_mut()` to reach the `WasiCtx` and `ResourceTable`.

---

## The Component — In Depth

```rust
let component = Component::from_file(engine, path)?;
```

A `Component` represents a compiled WebAssembly component. It is not yet running — it is just the compiled code and metadata, ready to be instantiated.

### What Component::from_file does

1. **Reads the `.wasm` file** from disk at the given path.
2. **Validates** that the bytes are a valid WebAssembly component (not a core module, not a corrupted file).
3. **Compiles** the WebAssembly bytecode using the Engine's compiler. Since this Engine targets Pulley, the compilation output is Pulley bytecode.
4. **Returns** the compiled `Component` object, which holds the compiled code and the component's type information (what it imports and exports).

### Where the .wasm files come from

Guest `.wasm` files are produced by running `cargo component build` inside each guest package. The `cargo-component` tool:

1. Compiles the Rust code with `--target wasm32-wasip1` (WebAssembly target with WASI support).
2. Reads the WIT file to understand the component's interface.
3. Packages the compiled core WebAssembly module as a Component Model component.
4. Writes the result to `target/wasm32-wasip1/debug/<name>.wasm`.

The host then loads these `.wasm` files at runtime.

### Component vs Module

In older WebAssembly code, you see `Module::from_file`. That loads a core WebAssembly module (numbers only, no WIT, no rich types). `Component::from_file` loads a Component Model component (rich types, WIT interfaces, proper string/option/record support). This project uses components exclusively.

---

## The Instance — In Depth

```rust
let instance = linker.instantiate(&mut store, &component)?;
```

An `Instance` is a running component. Instantiation connects a compiled Component to a Store and a Linker, resolving all imports.

### What instantiation does

1. **Resolves imports**: The component says "I need `wasi:cli/stdout`." The Linker says "I have `wasi:cli/stdout` registered." Wasmtime connects them.
2. **Allocates memory**: The component's linear memory is allocated inside the Store.
3. **Runs start functions**: If the component has initialization code, it runs now.
4. **Returns the Instance**: An object you can use to look up and call exported functions.

### If imports are missing

If the component imports something that the Linker does not provide, `instantiate` returns an error. This is why `add_to_linker_sync` must be called before instantiation — it registers all the WASI imports the guests need.

### Calling exports

After instantiation, you call exports through the Instance:

```rust
let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
run.call(&mut store, ())?;
```

**`get_typed_func::<(), ()>(&mut store, "run")`** — Look up an exported function named `"run"` and verify that it takes no parameters (first `()`) and returns nothing (second `()`). The type parameters are checked at this point. If the function's actual signature does not match, this call returns an error.

**`run.call(&mut store, ())`** — Execute the function. The `&mut store` is needed because the function may modify memory, call WASI imports (which access the Store's `WasiCtx`), or trap.

For guest2, the signature is different:

```rust
let run = instance.get_typed_func::<(Option<String>,), ()>(&mut store, "run")?;
run.call(&mut store, (Some(name.to_string()),))?;
```

`(Option<String>,)` is a one-element tuple. The Component Model's canonical ABI handles encoding the Rust `Option<String>` into the WebAssembly linear memory format that the guest expects. You write normal Rust types and the runtime handles the conversion.

---

## How They All Connect

Here is the dependency chain, top to bottom:

```
Config
  |
  v
Engine (holds compiled-code cache and compiler configuration)
  |
  +---> Component (compiled .wasm bytes, produced by Engine)
  |
  +---> Linker (import registry, associated with Engine)
  |       |
  |       +---> WASI imports registered via add_to_linker_sync
  |
  +---> Store (runtime state container)
          |
          +---> HostState (your struct: WasiCtx + ResourceTable)
          |
          +---> Instance (produced by linker.instantiate(&mut store, &component))
                  |
                  +---> Typed functions (produced by instance.get_typed_func)
                          |
                          +---> run.call() executes the guest code
```

Every arrow is a dependency. You cannot create a Store without an Engine. You cannot create a Linker without an Engine. You cannot instantiate a Component without both a Store and a Linker. The Engine is the root of everything.

---

## The Integration Tests — Line by Line

File: `tests/integration.rs`

The integration tests verify that the guest components work correctly without running the full host binary. They use the same Wasmtime APIs as `host.rs`.

### Test infrastructure

The tests define their own `TestHostState` (identical structure to `HostState` in `host.rs`) and helper functions:

- **`create_engine()`** — Creates an Engine with component-model support (but no Pulley target, since tests run natively).
- **`load_component(engine, path)`** — Loads a guest `.wasm` file using `Component::from_file`.
- **`build_test_linker(engine)`** — Creates a Linker with WASI imports registered.
- **`build_test_store(engine)`** — Creates a Store with inherited stdio.
- **`build_capture_store(engine)`** — Creates a Store with a `MemoryOutputPipe` capturing stdout instead of printing to the terminal. This lets tests check what the guest printed.

### What each test verifies

| Test                                           | What it checks                                                |
| ---------------------------------------------- | ------------------------------------------------------------- |
| `test_guest1_component_loads`                  | guest1.wasm is a valid component                              |
| `test_guest2_component_loads`                  | guest2.wasm is a valid component                              |
| `test_guest1_exports_run_function`             | guest1 exports `run` with signature `() -> ()`                |
| `test_guest2_exports_run_function`             | guest2 exports `run` with signature `(Option<String>,) -> ()` |
| `test_guest2_exports_describe_function`        | guest2 exports `describe` with signature `() -> (String,)`    |
| `test_guest1_does_not_export_describe`         | guest1 does NOT have a `describe` export                      |
| `test_guest1_run_executes_successfully`        | Calling guest1's `run` does not trap                          |
| `test_guest2_run_executes_successfully`        | Calling guest2's `run` with a parameter does not trap         |
| `test_guest2_describe_returns_expected_string` | `describe` returns the exact expected string                  |
| `test_guest1_imports_wasi`                     | guest1's component type declares WASI imports                 |
| `test_guest2_imports_wasi`                     | guest2's component type declares WASI imports                 |
| `test_guest1_run_produces_output`              | guest1's `run` writes text containing "guest1" to stdout      |
| `test_guest2_run_produces_output`              | guest2's `run` writes text containing "guest2" to stdout      |
| `test_guest2_run_default_name`                 | Passing `None` to guest2's `run` uses default "world"         |
| `test_guest2_run_custom_name`                  | Passing `Some("Pulley")` to guest2's `run` outputs "Pulley"   |

### How stdout capture works

```rust
fn build_capture_store(engine: &Engine) -> (Store<TestHostState>, MemoryOutputPipe) {
    let stdout = MemoryOutputPipe::new(4096);
    let state = TestHostState {
        ctx: WasiCtx::builder().stdout(stdout.clone()).build(),
        table: ResourceTable::new(),
    };
    (Store::new(engine, state), stdout)
}
```

Instead of `.inherit_stdio()` (which sends output to the terminal), this uses `.stdout(stdout.clone())` to redirect the guest's stdout into an in-memory buffer. After the guest runs, the test reads the buffer with `stdout.contents()` and checks the output with assertions. The `clone()` works because `MemoryOutputPipe` is backed by shared memory internally.

---

## How to Build and Run

### Prerequisites

Install Rust and `cargo-component`:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install cargo-component
```

### Build guest components

```
cargo component build --manifest-path guest1/Cargo.toml
cargo component build --manifest-path guest2/Cargo.toml
```

Each command compiles the guest Rust code to `wasm32-wasip1`, generates bindings from the WIT file, and packages the result as a component `.wasm` file.

### Run the host (default name)

```
cargo run --bin hello
```

Output:

```
Building Pulley component engine...
Compiling component from guest1/target/wasm32-wasip1/debug/guest1.wasm...
Instantiating and calling run...
guest1 run() called
Compiling component from guest2/target/wasm32-wasip1/debug/guest2.wasm...
Instantiating and calling run...
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
Compiling component from guest1/target/wasm32-wasip1/debug/guest1.wasm...
Instantiating and calling run...
guest1 run() called
Compiling component from guest2/target/wasm32-wasip1/debug/guest2.wasm...
Instantiating and calling run...
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

Microcontrollers like the RP2350 (Cortex-M33, ARMv8-M) use processor architectures that Cranelift's native code generator does not support. Pulley solves this by providing a portable interpreter that runs on any processor. The host firmware includes the Pulley interpreter, and guest components are compiled to Pulley bytecode.

### What the embedded version looks like

The [embedded-wasm-uart-rp2350](https://github.com/mytechnotalent/embedded-wasm-uart-rp2350) project uses the exact same architecture as this tutorial:

| This tutorial (laptop)                   | Embedded version (RP2350)                                   |
| ---------------------------------------- | ----------------------------------------------------------- |
| `host.rs` runs on your Mac/Linux/Windows | `src/main.rs` runs bare-metal on the RP2350                 |
| Config targets `pulley64`                | Config targets `pulley32`                                   |
| Guests use WASI for `println!()`         | Guests use custom WIT interfaces for UART I/O               |
| `.wasm` files loaded from disk           | `.wasm` precompiled and embedded in firmware flash          |
| Host calls `Component::from_file`        | Host calls `Component::deserialize` (pre-AOT-compiled)      |
| `WasiCtx` provides stdio                 | Custom `Host` trait provides `read_byte()` / `write_byte()` |

The mental model is identical. The host creates an Engine, compiles/loads a Component, builds a Linker with host-provided imports, creates a Store with host state, instantiates, and calls exports. The only differences are the target (`pulley32` vs `pulley64`), the imports (UART hardware vs WASI), and how the component bytes are loaded (embedded in flash vs read from disk).

### The key insight

This tutorial project teaches you the architecture. Once you understand Engine, Store, Linker, Component, and Instance here on your laptop, you understand exactly how it works on an embedded microcontroller. The concepts are the same. The only thing that changes is what hardware the host provides to the guest through WIT interfaces.

---

## Glossary

| Term                   | Definition                                                                                                                                   |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **WebAssembly (Wasm)** | A portable binary instruction format that runs inside a sandboxed virtual machine.                                                           |
| **Wasmtime**           | A production WebAssembly runtime written in Rust by the Bytecode Alliance.                                                                   |
| **Pulley**             | A portable interpreter inside Wasmtime that executes Pulley bytecode on any CPU, including embedded microcontrollers.                        |
| **WIT**                | WebAssembly Interface Types. A language for defining function signatures between host and guest.                                             |
| **Component Model**    | A standard on top of Wasm that adds rich types (strings, records, options) and WIT-based interfaces.                                         |
| **WASI**               | WebAssembly System Interface. Standardized APIs for OS-like capabilities (stdout, files, clocks).                                            |
| **Engine**             | The Wasmtime object that holds compiler configuration and the compiled-code cache. Created once, reused for all components.                  |
| **Config**             | Builder for Engine settings: target architecture, feature flags, memory limits.                                                              |
| **Store**              | The runtime state container. Holds your host state, WebAssembly memory, globals, and instance data. One per component instance.              |
| **HostState**          | Your custom struct stored inside the Store. Contains `WasiCtx` and `ResourceTable` so WASI imports can access host resources.                |
| **WasiCtx**            | The WASI context inside HostState. Configures what OS capabilities the guest is allowed to use (stdio, filesystem, etc.).                    |
| **ResourceTable**      | A table mapping integer handle IDs to host-side resources (files, sockets). Required by WASI even if unused.                                 |
| **WasiView**           | A trait implemented on HostState that tells wasmtime-wasi where to find `WasiCtx` and `ResourceTable`.                                       |
| **Linker**             | A registry of host-provided functions. Maps import names to implementations. Must satisfy all of a component's imports before instantiation. |
| **Component**          | A compiled WebAssembly component loaded from a `.wasm` file. Contains code and type metadata but is not yet running.                         |
| **Instance**           | A running component. Created by `linker.instantiate()`. You call exported functions through the Instance.                                    |
| **get_typed_func**     | Looks up an exported function by name and verifies its parameter/return types at lookup time.                                                |
| **call**               | Executes a typed function. The Pulley interpreter runs the guest code.                                                                       |
| **cargo-component**    | A Cargo subcommand that compiles Rust code to a Component Model `.wasm` file using WIT definitions.                                          |
| **bindings.rs**        | Auto-generated Rust code that bridges WIT types and Rust types. Created by cargo-component. Never edit by hand.                              |
| **Canonical ABI**      | The encoding format the Component Model uses to convert high-level types (strings, options, records) into bytes in linear memory.            |
| **Linear memory**      | A contiguous byte array inside the Store that the guest uses for heap allocations. Isolated from host memory.                                |
| **Cranelift**          | Wasmtime's native code compiler. Generates machine code for x86-64, AArch64, etc. Does not support embedded Cortex-M.                        |
| **AOT compilation**    | Ahead-Of-Time compilation. Compiling WebAssembly to Pulley bytecode before deployment so the embedded target does not need a compiler.       |
