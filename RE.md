# Reverse Engineering: Wasm (Desktop Pulley Host)

## Table of Contents

1. [Binary Overview](#1-binary-overview)
2. [Mach-O Header](#2-mach-o-header)
3. [Segment Layout](#3-segment-layout)
4. [Memory Map](#4-memory-map)
5. [Application Architecture](#5-application-architecture)
6. [Cranelift Compiler Integration](#6-cranelift-compiler-integration)
7. [Function Map](#7-function-map)
8. [Runtime Environment](#8-runtime-environment)
9. [Pulley Interpreter Deep Dive](#9-pulley-interpreter-deep-dive)
10. [Embedded cwasm Blobs](#10-embedded-cwasm-blobs)
11. [Host-Guest Call Flow](#11-host-guest-call-flow)
12. [RE Observations](#12-re-observations)
13. [Pulley Instruction Set Architecture](#13-pulley-instruction-set-architecture)
14. [Pulley Bytecode Disassembly](#14-pulley-bytecode-disassembly)
15. [Ghidra Analysis Walkthrough](#15-ghidra-analysis-walkthrough)

---

## 1. Binary Overview

| Property     | Value                       |
| ------------ | --------------------------- |
| File         | `hello`                     |
| Crate name   | `hello-world`               |
| Size on disk | 12,284,112 bytes (11.7 MiB) |
| Format       | Mach-O 64-bit executable    |
| Architecture | arm64 (Apple Silicon)       |
| Target OS    | macOS                       |
| Stripped     | No (27,450 symbols)         |

This is a **desktop** Wasmtime Component Model host that loads two
precompiled Pulley guest components (`guest1.cwasm`, `guest2.cwasm`)
from embedded bytes and executes their exported functions via the
**Pulley 64-bit** interpreter (`pulley64` target). Unlike the embedded
variants which run on bare-metal Cortex-M33, this binary runs on macOS
with full `std` support.

Key differences from the embedded-wasm variants:

| Aspect           | Embedded (RP2350)     | Desktop (macOS)               |
| ---------------- | --------------------- | ----------------------------- |
| Binary format    | ELF32 ARM             | Mach-O 64-bit arm64           |
| Pulley target    | `pulley32`            | `pulley64`                    |
| Guest count      | 1                     | 2 (guest1 + guest2)           |
| WIT interfaces   | Hardware (GPIO, UART) | Pure computation (no imports) |
| Binary size      | ~1.1-1.2 MiB          | 11.7 MiB                      |
| Cranelift linked | No                    | Yes (AOT compiler in binary)  |
| Hardware access  | Direct register I/O   | None (userspace process)      |

---

## 2. Mach-O Header

```
Mach header
      magic  cputype cpusubtype  caps    filetype ncmds sizeofcmds      flags
 0xfeedfacf 16777228          0  0x00           2    18       2160 0x00a18085
```

| Field     | Value                                  |
| --------- | -------------------------------------- |
| Magic     | `0xfeedfacf` (MH_MAGIC_64)             |
| CPU type  | `0x0100000c` (ARM64)                   |
| File type | `2` (MH_EXECUTE)                       |
| Load cmds | 18                                     |
| Flags     | `0x00a18085` (PIE, TWOLEVEL, DYLDLINK) |

The binary is a position-independent executable (PIE) with two-level
namespace bindings and dynamic linker support.

---

## 3. Segment Layout

```
Segment        VMAddr           VMSize       FileOff    FileSize     Prot   Description
__PAGEZERO     0x000000000      4 GiB        0          0            ---    Guard page
__TEXT         0x100000000      8.56 MiB     0          8,978,432    r-x    Code + constants
__DATA_CONST   0x100890000      384 KiB      8,978,432  393,216      rw-    Const data
__DATA         0x1008f0000      16 KiB       9,371,648  16,384       rw-    Mutable data
__LINKEDIT     0x1008f4000      2.77 MiB     9,388,032  2,896,080    r--    Symbol tables
```

### __TEXT Section Breakdown

| Section            | Size     | Description                  |
| ------------------ | -------- | ---------------------------- |
| `__text`           | 6.89 MiB | All executable code          |
| `__const`          | 703 KiB  | Read-only data + cwasm blobs |
| `__gcc_except_tab` | 263 KiB  | Exception handling tables    |
| `__unwind_info`    | 142 KiB  | Stack unwinding metadata     |
| `__cstring`        | 85 KiB   | C string literals            |
| `__stubs`          | 1.2 KiB  | Lazy symbol stubs            |
| `__stub_helper`    | 1.2 KiB  | Stub helper code             |

### Size Analysis

The binary is **10x larger** than the embedded variants due to:

1. **Cranelift compiler** (~5 MiB): The full AOT compiler is linked in
   for `Engine::new()` even though we only deserialize precompiled
   components at runtime.

2. **Exception handling** (405 KiB): `__gcc_except_tab` + `__unwind_info`
   for Rust panic unwinding support (not available in `no_std`).

3. **Two cwasm blobs**: Both `guest1.cwasm` and `guest2.cwasm` are
   embedded in `__const`.

4. **Standard library**: Full `std` linked (threads, I/O, allocator).

---

## 4. Memory Map

```
Virtual Address Space (arm64 macOS):

0x000000000 - 0x0FFFFFFFF   __PAGEZERO (4 GiB guard)
0x100000000 - 0x10088FFFF   __TEXT (code + constants)
    +-- 0x1000008c0          __text starts (executable code)
    +-- 0x10068e934          __stubs (dyld lazy binding)
    +-- 0x1006cfce0          __const (cwasm blobs embedded here)
    +-- 0x10077b8c8          __cstring (string literals)
0x100890000 - 0x1008EFFFF   __DATA_CONST (vtables, type metadata)
0x1008f0000 - 0x1008F3FFF   __DATA (mutable globals, TLS)
0x1008f4000 - 0x100BB7FFF   __LINKEDIT (symbols, string table)

Heap:   Managed by system allocator (malloc/free)
Stack:  Thread stack managed by macOS (default 8 MiB)
```

---

## 5. Application Architecture

### 5.1 Entry Point

The macOS dynamic linker (`dyld`) calls `_main` which dispatches to the
Rust runtime entry point. The application entry is `hello::main` at
`0x10002096c`.

### 5.2 Execution Flow

```
_main (dyld entry)
  -> std::rt::lang_start
    -> hello::main (0x10002096c)
      -> hello::run()                [inlined]
        -> parse_name()              [inlined] — read argv[1] or "Pulley"
        -> build_engine()            [inlined] — Config + pulley64 target
        -> load_component(guest1)    [inlined] — deserialize guest1.cwasm
        -> run_guest1()              [inlined] — instantiate + call run()
        -> load_component(guest2)    [inlined] — deserialize guest2.cwasm
        -> run_guest2(name)          [inlined] — instantiate + call run(name) + describe()
```

All application functions (`build_engine`, `load_component`, `run_guest1`,
`run_guest2`, `parse_name`, `run`) are inlined into `hello::main` by the
optimizer.

### 5.3 Engine Configuration

```rust
let mut config = Config::new();
config.wasm_component_model(true);
config.target("pulley64")?;
Engine::new(&config)
```

Unlike the embedded variants which disable OS-dependent features
(`signals_based_traps(false)`, `memory_init_cow(false)`, etc.), the
desktop engine uses default settings. The `pulley64` target selects the
64-bit Pulley ISA variant with 64-bit pointer widths.

---

## 6. Cranelift Compiler Integration

The desktop binary includes the full **Cranelift** code generator —
the single largest component by code size. It is linked because
`Engine::new()` compiles the AOT pipeline, even though our usage only
calls `Component::deserialize()` with precompiled cwasm bytes.

### 6.1 Top Cranelift Functions by Size

| Address       | Size      | Demangled Name                                   |
| ------------- | --------- | ------------------------------------------------ |
| `0x1003da46c` | 168,192 B | `constructor_simplify` (egraph optimization)     |
| `0x100390cac` | 56,696 B  | `pulley_shared::inst::print` (instruction print) |
| `0x1004e94a0` | 56,264 B  | `aarch64::constructor_lower` (ARM64 lowering)    |
| `0x1002b9fe4` | 36,884 B  | `pulley_shared::constructor_lower` (Pulley)      |
| `0x1002b0fd0` | 36,884 B  | `pulley_shared::constructor_lower` (Pulley #2)   |
| `0x100232aec` | 32,284 B  | `translate_operator` (Wasm-to-CLIF)              |
| `0x1004dfbe8` | 28,776 B  | `aarch64::MInst::print_with_state`               |

### 6.2 Cranelift Architecture

The binary includes lowering backends for **both** the host (aarch64)
and the Pulley target:

- `cranelift_codegen::isa::aarch64` — ARM64 native code generation
- `cranelift_codegen::isa::pulley_shared` — Pulley bytecode emission

Both are used during the `build.rs` AOT compilation step. At runtime
only the Pulley interpreter path is active.

---

## 7. Function Map

### 7.1 Application Functions

| Address       | Symbol        | Purpose                           |
| ------------- | ------------- | --------------------------------- |
| `0x10002096c` | `hello::main` | Entry point (all helpers inlined) |

All other application functions (`build_engine`, `load_component`,
`run_guest1`, `run_guest2`, `parse_name`, `run`) are inlined into
`hello::main` by the release-mode optimizer.

### 7.2 Wasmtime Runtime (Key Symbols)

| Address       | Demangled Name                            |
| ------------- | ----------------------------------------- |
| `0x10009ab08` | `OperatorCost::deserialize`               |
| `0x10050a75c` | `decode_one_extended`                     |
| `0x100510158` | `Interpreter::run` (Pulley dispatch loop) |
| `0x10053b3d4` | `OperatorCost::cost`                      |
| `0x10053d488` | `OperatorCostStrategy::cost`              |
| `0x1000c356c` | `Metadata::check_cost`                    |
| `0x100045078` | `InterpreterRef::call`                    |
| `0x100513708` | `Vm::call_start`                          |
| `0x100513998` | `Vm::call_run`                            |

### 7.3 Symbol Statistics

| Category             | Count  | % of 27,450 |
| -------------------- | ------ | ----------- |
| Cranelift codegen    | ~8,000 | 29%         |
| Wasmtime runtime     | ~6,000 | 22%         |
| Pulley interpreter   | ~2,000 | 7%          |
| Standard library     | ~5,000 | 18%         |
| Component model/WIT  | ~3,000 | 11%         |
| Other (alloc, serde) | ~3,450 | 13%         |

---

## 8. Runtime Environment

### 8.1 Dynamic Libraries

The binary links against macOS system libraries:

```
/usr/lib/libSystem.B.dylib       — POSIX syscalls, malloc, pthreads
/usr/lib/libc++.1.dylib          — C++ standard library (for exception handling)
```

### 8.2 Memory Management

- **Heap**: System allocator (`malloc`/`free`) — no custom allocator
- **Stack**: Default macOS thread stack (8 MiB)
- **Wasm linear memory**: Allocated by Wasmtime's `MemoryCreator` using
  `mmap` with guard pages

### 8.3 No Hardware Interaction

Unlike the embedded variants which directly access SIO, UART, and
SysTick registers, this binary has **no hardware register access**. All
I/O goes through standard library functions (`println!`, `std::env::args`).

---

## 9. Pulley Interpreter Deep Dive

### 9.1 Interpreter Entry (`InterpreterRef::call`)

```
Location:  0x100045078
```

Call sequence (for guest1):

```
hello::main()
  -> Engine::new(&config)           ; pulley64 target
  -> Component::deserialize(guest1.cwasm)
  -> Linker::instantiate()
  -> get_typed_func("run")
  -> run.call()
    -> InterpreterRef::call()       <-- native-to-Pulley boundary
      -> Vm::call_start()           ; Set up Pulley register file
      -> Vm::call_run()             ; Enter interpreter loop
        -> Interpreter::run()       ; Main dispatch loop
```

### 9.2 Main Dispatch Loop

```
Location:  0x100510158
```

Same two-level dispatch scheme as the embedded variants. The 64-bit
variant uses 64-bit registers and pointers in the Pulley register file,
but the dispatch mechanism is identical.

### 9.3 Pulley 64-bit vs 32-bit

| Aspect             | pulley32 (embedded)  | pulley64 (desktop)   |
| ------------------ | -------------------- | -------------------- |
| Register width     | 32-bit               | 64-bit               |
| Pointer size       | 4 bytes              | 8 bytes              |
| Linear memory addr | 32-bit offsets       | 64-bit offsets       |
| Load/store ops     | `xload32le_*`        | `xload64le_*`        |
| cwasm ELF target   | pulley32-unknown-elf | pulley64-unknown-elf |

---

## 10. Embedded cwasm Blobs

### 10.1 Two Guest Components

The binary embeds two precompiled cwasm blobs via `include_bytes!`:

```rust
const GUEST1_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest1.cwasm"));
const GUEST2_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest2.cwasm"));
```

Both are located in the `__const` section of `__TEXT` segment at
`0x1006cfce0`.

### 10.2 Guest1 — Simple Export

**WIT contract** (`guest1/wit/world.wit`):

```wit
package component:guest1;
world guest1-world {
    export run: func() -> string;
}
```

**Guest code**:

```rust
fn run() -> String {
    String::from("guest1 run() called")
}
```

No host imports — the guest is a pure function that returns a string.

### 10.3 Guest2 — Parameterized Export + Describe

**WIT contract** (`guest2/wit/world.wit`):

```wit
package component:guest2;
world guest2-world {
    export run: func(name: option<string>) -> string;
    export describe: func() -> string;
}
```

**Guest code**:

```rust
fn run(name: Option<String>) -> String {
    let name = name.as_deref().unwrap_or("world");
    format!("guest2 run() called: hello, {name}!")
}

fn describe() -> String {
    String::from("guest2 has an extra `describe` export")
}
```

Guest2 has two exports and accepts an optional string parameter. The
`format!` macro generates Pulley bytecode that performs string
concatenation through the guest's `dlmalloc` allocator.

---

## 11. Host-Guest Call Flow

### 11.1 Guest1 Execution

```
[arm64 Native]  hello::main
    |  Engine::new(pulley64)
    |  Component::deserialize(GUEST1_PRECOMPILED)
    |  Linker::instantiate()
    v
[arm64 Native]  get_typed_func::<(), (String,)>("run")
    |  run.call(())
    v
[arm64 Native]  InterpreterRef::call (0x100045078)
    |  Set up Pulley64 register file
    v
[Pulley64 VM]   Guest1::run()
    |  Allocate string "guest1 run() called" via dlmalloc
    |  Return string through component model canonical ABI
    v
[arm64 Native]  Receive String result
    |  println!("{result}")
```

### 11.2 Guest2 Execution

```
[arm64 Native]  hello::main
    |  Component::deserialize(GUEST2_PRECOMPILED)
    |  Linker::instantiate()
    v
[arm64 Native]  get_typed_func::<(Option<String>,), (String,)>("run")
    |  run.call((Some("Pulley"),))
    v
[Pulley64 VM]   Guest2::run(Some("Pulley"))
    |  Unwrap option -> "Pulley"
    |  format!("guest2 run() called: hello, Pulley!")
    |  Return formatted string
    v
[arm64 Native]  println!("{run_result}")
    |
    v
[arm64 Native]  get_typed_func::<(), (String,)>("describe")
    |  describe.call(())
    v
[Pulley64 VM]   Guest2::describe()
    |  Return "guest2 has an extra `describe` export"
    v
[arm64 Native]  println!("describe: {desc_result}")
```

### 11.3 No Host Imports

Unlike the embedded variants where guests call back into host code for
hardware access (GPIO, UART, SysTick), these desktop guests have **no
imports**. All WIT declarations are exports only. The Pulley interpreter
runs the guest code to completion without any host callbacks.

---

## 12. RE Observations

### 12.1 Binary Composition

| Component                 | Approx Size | % of __text |
| ------------------------- | ----------- | ----------- |
| Cranelift codegen         | ~3.5 MiB    | 50.8%       |
| Wasmtime runtime          | ~1.5 MiB    | 21.8%       |
| Pulley interpreter        | ~0.5 MiB    | 7.3%        |
| Component model / WIT     | ~0.5 MiB    | 7.3%        |
| Standard library          | ~0.6 MiB    | 8.7%        |
| Application code          | <1 KiB      | <0.1%       |
| Exception handling        | ~263 KiB    | 3.7%        |
| Other (serde, alloc, etc) | ~0.3 MiB    | 4.4%        |

### 12.2 Dead Code: Cranelift

The Cranelift compiler accounts for ~50% of the binary but is **not used
at runtime**. The application only calls `Component::deserialize()` which
loads precompiled Pulley bytecode. Cranelift is linked because
`Engine::new()` pulls in the entire compilation pipeline. A future
optimization could use `wasmtime`'s `cranelift` feature flag more
selectively.

### 12.3 Desktop vs Embedded Comparison

| Metric             | Desktop (`hello`) | Embedded (blinky)   |
| ------------------ | ----------------- | ------------------- |
| Binary size        | 11.7 MiB          | 1.12 MiB            |
| Code size (__text) | 6.89 MiB          | 533 KiB             |
| Functions          | 27,450            | 2,375               |
| Application code   | <1 KiB            | ~6 KiB              |
| Cranelift linked   | Yes (~3.5 MiB)    | No (0 B)            |
| Pulley word size   | 64-bit            | 32-bit              |
| WIT imports        | 0                 | 2-4 (gpio, etc.)    |
| Guest components   | 2                 | 1                   |
| Hardware access    | None              | Direct register I/O |
| OS support         | Full std          | no_std bare-metal   |
| Heap allocator     | System malloc     | embedded-alloc LLFF |

### 12.4 Key Addresses Quick Reference

| Address       | What                                         |
| ------------- | -------------------------------------------- |
| `0x1000008c0` | __text section start                         |
| `0x10002096c` | hello::main (application entry)              |
| `0x100045078` | InterpreterRef::call (native->Pulley bridge) |
| `0x100510158` | Pulley Interpreter::run (dispatch loop)      |
| `0x10050a75c` | Pulley decode_one_extended                   |
| `0x100513708` | Vm::call_start                               |
| `0x100513998` | Vm::call_run                                 |
| `0x10009ab08` | OperatorCost::deserialize                    |
| `0x1006cfce0` | __const section (cwasm blobs embedded here)  |
| `0x1003da46c` | Cranelift constructor_simplify (168 KiB)     |
| `0x100390cac` | Cranelift pulley inst::print (57 KiB)        |
| `0x1004e94a0` | Cranelift aarch64 constructor_lower (56 KiB) |

---

## 13. Pulley Instruction Set Architecture

### 13.1 Overview

Pulley is Wasmtime's portable bytecode interpreter (wasmtime 43.0.0,
`pulley-interpreter` crate v43.0.0). It defines a register-based ISA
with variable-length instructions, designed for efficient interpretation
rather than native execution.

### 13.2 Encoding Format

**Primary opcodes** use a 1-byte opcode followed by operands:

```
[opcode:1] [operands:0-9]
```

There are **220 primary opcodes** (0x00-0xDB). Opcode `0xDC` is the
**ExtendedOp** sentinel — when the interpreter encounters it, it reads
a 2-byte extended opcode:

```
[0xDC] [ext_opcode:2] [operands:0-N]
```

There are **310 extended opcodes** (0x0000-0x0135) for SIMD, float
conversions, and complex operations.

### 13.3 64-bit Variant

The `pulley64` target uses the same opcode encoding as `pulley32` but
with 64-bit general-purpose registers and pointer-width operations.
Load/store instructions like `xload64le_*` and address computations use
full 64-bit values.

See the [embedded-wasm-servo-rp2350 RE.md](https://github.com/mytechnotalent/embedded-wasm-servo-rp2350)
§13 for the complete Pulley ISA reference.

---

## 14. Pulley Bytecode Disassembly

### 14.1 Guest1::run() — String Return

```
; function: Guest1::run() -> string

push_frame_save <frame>, <callee-saved regs>

; Load VMContext
xload64le_o32 x_heap, x0, ...        ; heap_base (64-bit pointer)
xmov x_vmctx, x0

; Allocate string via dlmalloc
; "guest1 run() called" (20 bytes)
xconst8 x_len, 20                    ; string length
call <cabi_realloc>                   ; allocate 20 bytes on guest heap

; Copy string data to allocated buffer
; ... memcpy of "guest1 run() called" ...

; Return (ptr, len) through canonical ABI
xmov x_ret_ptr, x_alloc
xmov x_ret_len, x_len
pop_frame_restore <frame>, <callee-saved regs>
ret
```

### 14.2 Guest2::run(name) — String Formatting

```
; function: Guest2::run(name: option<string>) -> string

push_frame_save <frame>, <callee-saved regs>

; Load parameters via canonical ABI
; x2 = discriminant (0 = None, 1 = Some)
; x3, x4 = string ptr, len (if Some)

; Check if name is Some
br_if_xeq32 x2, x_zero, .use_default

; Some case: use provided name
xmov x_name_ptr, x3
xmov x_name_len, x4
jump .format

.use_default:
; None case: use "world"
xconst8 x_name_len, 5
; load address of "world" literal
; ...

.format:
; Concatenate: "guest2 run() called: hello, " + name + "!"
; Multiple cabi_realloc calls for string building
; ...

pop_frame_restore <frame>, <callee-saved regs>
ret
```

---

## 15. Ghidra Analysis Walkthrough

### 15.1 Import and Initial Analysis

1. **File -> Import File**: Select the `hello` binary. Ghidra
   auto-detects `AARCH64:LE:64:AppleSilicon`. Accept the defaults.

2. **Auto-analysis**: Ghidra identifies approximately 27,450 functions
   from the Mach-O symbol table.

3. **Analysis time**: ~2-3 minutes for this 11.7 MiB binary.

### 15.2 Symbol Tree Navigation

```
Functions/ (27,450 total)
+-- _main                                          (dyld entry)
+-- hello::main                                    0x10002096c
+-- pulley_interpreter::interp::Interpreter::run   0x100510158
+-- pulley_interpreter::decode::decode_one_extended 0x10050a75c
+-- InterpreterRef::call                           0x100045078
+-- Vm::call_start                                 0x100513708
+-- Vm::call_run                                   0x100513998
+-- cranelift_codegen::opts::constructor_simplify   0x1003da46c
+-- cranelift_codegen::isa::pulley_shared::lower    0x1002b9fe4
+-- ... (27,441 more)
```

### 15.3 Finding the cwasm Blobs

1. Search for `\x7fELF` in the `__const` section (starting at
   `0x1006cfce0`)
2. Both `guest1.cwasm` and `guest2.cwasm` are located here
3. Each starts with ELF64 magic and targets `pulley64-unknown-unknown-elf`
4. Right-click -> **Select Bytes** -> export each blob separately

### 15.4 Ghidra + G-Pulley: Full-Stack Analysis

With the [G-Pulley](https://github.com/mytechnotalent/G-Pulley) extension
installed, Ghidra can analyze **both** the arm64 host binary and the
Pulley guest bytecode:

| Aspect                  | arm64 Host Code           | Pulley Guest Code (G-Pulley)        |
| ----------------------- | ------------------------- | ----------------------------------- |
| Disassembly             | Full AArch64              | Full Pulley ISA mnemonics           |
| Function identification | Automatic from symbols    | Automatic (cwasm loader + analyzer) |
| Cross-references        | Full xref graph           | Function calls and branches         |
| Control flow            | CFG with switch detection | Branch and jump targets resolved    |

**G-Pulley provides**:

- Custom ELF loader that extracts `.cwasm` blobs from the host binary
- SLEIGH processor spec for Pulley 32-bit and 64-bit ISA (Wasmtime v43.0.0)
- Post-load analyzer that discovers functions, trampolines, and host calls
- Full opcode decoding for all 220 primary + 310 extended Pulley opcodes

### 15.5 Recommended Ghidra Workflow

1. **Install G-Pulley**: Download from
   [G-Pulley releases](https://github.com/mytechnotalent/G-Pulley/releases).
   In Ghidra: **File -> Install Extensions -> + -> select zip**. Restart.

2. **Analyze the arm64 host**: Import the Mach-O binary. Run
   auto-analysis. Navigate to `hello::main` (0x10002096c) to see the
   inlined application logic — engine creation, component deserialization,
   guest invocation.

3. **Trace the interpreter**: Start at `InterpreterRef::call`
   (0x100045078), follow into `Vm::call_start` -> `Vm::call_run` ->
   `Interpreter::run` (0x100510158) to see the Pulley dispatch loop.

4. **Examine Cranelift**: Navigate to the large Cranelift functions
   (`constructor_simplify` at 0x1003da46c) to understand the AOT
   compilation pipeline, though these are not exercised at runtime.

5. **Analyze the Pulley bytecode**: Import the host binary again using
   G-Pulley's cwasm loader (select "Pulley cwasm" format). G-Pulley
   extracts the embedded cwasm blobs, disassembles all Pulley opcodes,
   and identifies guest functions including `Guest1::run()` and
   `Guest2::run()` with their string allocation and formatting logic.
