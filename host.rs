//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Component-Model Host Running on Pulley
//!
//! Loads AOT-precompiled WebAssembly Component Model guest artifacts from
//! embedded bytes, instantiates them, and executes exported functions via
//! the Pulley interpreter backend. Guests are `no_std` components compiled
//! to `wasm32-unknown-unknown` with WIT contracts in `guest1/wit/world.wit`
//! and `guest2/wit/world.wit`.

use wasmtime::component::{Component, Linker}; // Component Model loader and linker.
use wasmtime::{Config, Engine, Result, Store}; // Wasmtime runtime core types.

/// Precompiled Pulley bytecode for guest1, embedded at build time by `build.rs`.
const GUEST1_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest1.cwasm"));

/// Precompiled Pulley bytecode for guest2, embedded at build time by `build.rs`.
const GUEST2_PRECOMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest2.cwasm"));

/// Default name passed to guests that accept an `Option<String>` parameter.
const DEFAULT_GUEST_NAME: &str = "Pulley";

/// Builds a Wasmtime engine configured for Component Model + Pulley.
///
/// # Returns
///
/// A configured `Engine` that enables component-model support and targets
/// `pulley64`.
fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target("pulley64")?;
    Engine::new(&config)
}

/// Deserializes a precompiled component from embedded bytes.
///
/// # Safety
///
/// Uses `unsafe` to call `Component::deserialize`, which assumes the byte
/// buffer is trusted serialized Wasmtime component code. This invariant is
/// upheld by `build.rs`, which produces these bytes via Wasmtime.
///
/// # Arguments
///
/// * `engine` - Runtime engine configured identically to build-time settings.
/// * `bytes` - Precompiled Pulley component bytes.
///
/// # Returns
///
/// A deserialized `Component` ready for instantiation.
fn load_component(engine: &Engine, bytes: &[u8]) -> Result<Component> {
    unsafe { Component::deserialize(engine, bytes) }
}

/// Instantiates guest1 and calls its exported `run` function.
///
/// # Arguments
///
/// * `engine` - Runtime engine.
/// * `component` - Deserialized guest1 Pulley component.
///
/// # Returns
///
/// The string returned by guest1's `run` export.
fn run_guest1(engine: &Engine, component: &Component) -> Result<String> {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let run = instance.get_typed_func::<(), (String,)>(&mut store, "run")?;
    let (result,) = run.call(&mut store, ())?;
    Ok(result)
}

/// Instantiates guest2 and calls its exported `run` and `describe` functions.
///
/// # Arguments
///
/// * `engine` - Runtime engine.
/// * `component` - Deserialized guest2 Pulley component.
/// * `name` - Name passed to guest2's `run` export.
///
/// # Returns
///
/// A tuple of the `run` result string and the `describe` result string.
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

/// Parses the target name from CLI arguments.
///
/// # Returns
///
/// The first positional argument or `DEFAULT_GUEST_NAME` when omitted.
fn parse_name() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_GUEST_NAME.to_string())
}

/// Runs the full host flow for both precompiled component artifacts.
///
/// # Returns
///
/// `Ok(())` when both components execute successfully.
fn run() -> Result<()> {
    let name = parse_name();
    println!("Building Pulley component engine...");
    let engine = build_engine()?;
    println!("Deserializing guest1 component...");
    let guest1 = load_component(&engine, GUEST1_PRECOMPILED)?;
    let result = run_guest1(&engine, &guest1)?;
    println!("{result}");
    println!("Deserializing guest2 component...");
    let guest2 = load_component(&engine, GUEST2_PRECOMPILED)?;
    let (run_result, desc_result) = run_guest2(&engine, &guest2, &name)?;
    println!("{run_result}");
    println!("describe: {desc_result}");
    println!("Done.");
    Ok(())
}

/// Program entry point.
///
/// # Returns
///
/// `Ok(())` when the host runs successfully.
fn main() -> Result<()> {
    run()
}
