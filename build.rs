//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Build Script for Component-Model Host with Pulley
//!
//! Compiles both guest wasm crates, encodes them as WebAssembly components,
//! AOT-precompiles them to Pulley bytecode, and writes the serialized
//! artifacts to `OUT_DIR` for host-side `include_bytes!`.

use std::path::{Path, PathBuf};
use std::process::Command;
use wasmtime::{Config, Engine};
use wit_component::ComponentEncoder;

/// Guest1 core wasm path produced by the guest build.
const GUEST1_CORE_WASM_PATH: &str = "guest1/target/wasm32-unknown-unknown/release/guest1.wasm";

/// Guest2 core wasm path produced by the guest build.
const GUEST2_CORE_WASM_PATH: &str = "guest2/target/wasm32-unknown-unknown/release/guest2.wasm";

/// Serialized Pulley component file name for guest1 emitted to `OUT_DIR`.
const GUEST1_SERIALIZED_NAME: &str = "guest1.cwasm";

/// Serialized Pulley component file name for guest2 emitted to `OUT_DIR`.
const GUEST2_SERIALIZED_NAME: &str = "guest2.cwasm";

/// Encoded component file name for guest1 emitted to `OUT_DIR` (for tests).
const GUEST1_COMPONENT_NAME: &str = "guest1.component.wasm";

/// Encoded component file name for guest2 emitted to `OUT_DIR` (for tests).
const GUEST2_COMPONENT_NAME: &str = "guest2.component.wasm";

/// Creates the build output directory path from Cargo environment variables.
///
/// # Returns
///
/// Cargo's build output directory path.
///
/// # Panics
///
/// Panics if `OUT_DIR` is not available.
fn output_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR not set"))
}

/// Compiles a guest crate to a core wasm module.
///
/// # Arguments
///
/// * `guest_dir` - Directory containing the guest crate.
///
/// # Panics
///
/// Panics if the guest build command fails or exits non-zero.
fn compile_guest_wasm(guest_dir: &str) {
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(guest_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .expect("failed to build guest wasm crate");
    assert!(status.success(), "guest wasm build failed: {}", guest_dir);
}

/// Creates a Wasmtime engine configured for Pulley64 component precompilation.
///
/// # Returns
///
/// A configured Pulley64 `Engine`.
///
/// # Panics
///
/// Panics if the target cannot be configured or the engine cannot be created.
fn pulley_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target("pulley64").expect("set pulley64 target");
    Engine::new(&config).expect("create pulley engine")
}

/// Reads guest core wasm bytes from disk.
///
/// # Arguments
///
/// * `path` - Filesystem path to the core wasm binary.
///
/// # Returns
///
/// The raw core wasm bytes.
///
/// # Panics
///
/// Panics if the guest core wasm cannot be read.
fn read_core_wasm(path: &str) -> Vec<u8> {
    std::fs::read(path).expect("read guest core wasm")
}

/// Encodes core wasm bytes into a WebAssembly component binary.
///
/// # Arguments
///
/// * `wasm_bytes` - Raw guest core wasm bytes.
///
/// # Returns
///
/// Encoded component bytes.
///
/// # Panics
///
/// Panics if component encoding fails.
fn encode_component(wasm_bytes: &[u8]) -> Vec<u8> {
    ComponentEncoder::default()
        .module(wasm_bytes)
        .expect("set core wasm module")
        .validate(true)
        .encode()
        .expect("encode component")
}

/// Precompiles component bytes to Pulley64 serialized bytecode.
///
/// # Arguments
///
/// * `engine` - Pulley64 engine for precompilation.
/// * `component_bytes` - Encoded WebAssembly component bytes.
///
/// # Returns
///
/// Serialized Pulley bytecode bytes.
///
/// # Panics
///
/// Panics if precompilation fails.
fn precompile_component(engine: &Engine, component_bytes: &[u8]) -> Vec<u8> {
    engine
        .precompile_component(component_bytes)
        .expect("precompile component for pulley64")
}

/// Writes precompiled Pulley component bytes to `OUT_DIR`.
///
/// # Arguments
///
/// * `out_dir` - Cargo output directory.
/// * `name` - Output file name.
/// * `bytes` - Serialized Pulley component bytes.
///
/// # Panics
///
/// Panics if writing fails.
fn write_precompiled_component(out_dir: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(out_dir.join(name), bytes).expect("write precompiled pulley component");
}

/// Compiles a guest, encodes as a component, and precompiles to Pulley bytecode.
///
/// Also writes the encoded component bytes (pre-AOT) for integration tests.
///
/// # Arguments
///
/// * `engine` - Pulley64 engine for precompilation.
/// * `out_dir` - Cargo output directory where serialized output is written.
/// * `guest_dir` - Directory containing the guest crate.
/// * `core_wasm_path` - Path to the core wasm binary produced by the guest build.
/// * `output_name` - File name for the serialized Pulley component.
/// * `component_name` - File name for the encoded component (for tests).
fn compile_guest_to_pulley(
    engine: &Engine,
    out_dir: &Path,
    guest_dir: &str,
    core_wasm_path: &str,
    output_name: &str,
    component_name: &str,
) {
    compile_guest_wasm(guest_dir);
    let core_wasm = read_core_wasm(core_wasm_path);
    let component = encode_component(&core_wasm);
    write_precompiled_component(out_dir, component_name, &component);
    let serialized = precompile_component(engine, &component);
    write_precompiled_component(out_dir, output_name, &serialized);
}

/// Registers the files that should trigger build script reruns.
fn print_rerun_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=guest1/wit/world.wit");
    println!("cargo:rerun-if-changed=guest1/src/lib.rs");
    println!("cargo:rerun-if-changed=guest1/Cargo.toml");
    println!("cargo:rerun-if-changed=guest2/wit/world.wit");
    println!("cargo:rerun-if-changed=guest2/src/lib.rs");
    println!("cargo:rerun-if-changed=guest2/Cargo.toml");
}

/// Build script entry point that compiles both guests and precompiles to Pulley.
fn main() {
    let out_dir = output_dir();
    let engine = pulley_engine();
    compile_guest_to_pulley(
        &engine,
        &out_dir,
        "guest1",
        GUEST1_CORE_WASM_PATH,
        GUEST1_SERIALIZED_NAME,
        GUEST1_COMPONENT_NAME,
    );
    compile_guest_to_pulley(
        &engine,
        &out_dir,
        "guest2",
        GUEST2_CORE_WASM_PATH,
        GUEST2_SERIALIZED_NAME,
        GUEST2_COMPONENT_NAME,
    );
    print_rerun_triggers();
}
