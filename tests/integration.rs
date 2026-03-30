//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Integration Tests for Component-Model Guests on Pulley
//!
//! Validates that the compiled WASM guest components load correctly through
//! the Component Model, instantiate without WASI, export the expected
//! functions (`run` and optionally `describe`), and return correct values
//! when executed via the default engine.

use wasmtime::component::{Component, Linker}; // Component Model loader and linker.
use wasmtime::{Engine, Store}; // Wasmtime runtime core types.

/// Compiled guest1 component embedded at build time.
const GUEST1_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest1.component.wasm"));

/// Compiled guest2 component embedded at build time.
const GUEST2_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest2.component.wasm"));

/// Creates a default wasmtime engine.
///
/// # Returns
///
/// A wasmtime `Engine` with default configuration.
fn create_engine() -> Engine {
    Engine::default()
}

/// Compiles an embedded WASM binary into a wasmtime component.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine to compile with.
/// * `wasm` - Raw component WASM bytes.
///
/// # Returns
///
/// The compiled WASM `Component`.
///
/// # Panics
///
/// Panics if the WASM binary is invalid.
fn compile_component(engine: &Engine, wasm: &[u8]) -> Component {
    Component::new(engine, wasm).expect("valid WASM component")
}

/// Instantiates a component and calls its `run` export with no arguments.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine.
/// * `component` - The compiled WASM component.
///
/// # Returns
///
/// The string returned by the component's `run` export.
///
/// # Panics
///
/// Panics if instantiation or the `run` call fails.
fn call_run_no_args(engine: &Engine, component: &Component) -> String {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker
        .instantiate(&mut store, component)
        .expect("instantiate");
    let run = instance
        .get_typed_func::<(), (String,)>(&mut store, "run")
        .expect("get run");
    let (result,) = run.call(&mut store, ()).expect("call run");
    result
}

/// Instantiates a component and calls its `run` export with an optional name.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine.
/// * `component` - The compiled WASM component.
/// * `name` - Optional name to pass to the `run` export.
///
/// # Returns
///
/// The string returned by the component's `run` export.
///
/// # Panics
///
/// Panics if instantiation or the `run` call fails.
fn call_run_with_name(engine: &Engine, component: &Component, name: Option<&str>) -> String {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker
        .instantiate(&mut store, component)
        .expect("instantiate");
    let run = instance
        .get_typed_func::<(Option<String>,), (String,)>(&mut store, "run")
        .expect("get run");
    let (result,) = run
        .call(&mut store, (name.map(String::from),))
        .expect("call run");
    result
}

/// Instantiates a component and calls its `describe` export.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine.
/// * `component` - The compiled WASM component.
///
/// # Returns
///
/// The string returned by the component's `describe` export.
///
/// # Panics
///
/// Panics if instantiation or the `describe` call fails.
fn call_describe(engine: &Engine, component: &Component) -> String {
    let linker = Linker::<()>::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker
        .instantiate(&mut store, component)
        .expect("instantiate");
    let describe = instance
        .get_typed_func::<(), (String,)>(&mut store, "describe")
        .expect("get describe");
    let (result,) = describe.call(&mut store, ()).expect("call describe");
    result
}

/// Verifies that the guest1 component binary loads without error.
///
/// # Panics
///
/// Panics if the guest1 component binary fails to compile.
#[test]
fn test_guest1_component_loads() {
    let engine = create_engine();
    let _component = compile_component(&engine, GUEST1_WASM);
}

/// Verifies that the guest2 component binary loads without error.
///
/// # Panics
///
/// Panics if the guest2 component binary fails to compile.
#[test]
fn test_guest2_component_loads() {
    let engine = create_engine();
    let _component = compile_component(&engine, GUEST2_WASM);
}

/// Verifies that guest1 instantiates and exports the `run` function.
///
/// # Panics
///
/// Panics if the component fails to instantiate or `run` is not found.
#[test]
fn test_guest1_exports_run_function() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST1_WASM);
    let linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let run = instance.get_typed_func::<(), (String,)>(&mut store, "run");
    assert!(run.is_ok(), "guest1 must export `run`");
}

/// Verifies that guest2 instantiates and exports the `run` function.
///
/// # Panics
///
/// Panics if the component fails to instantiate or `run` is not found.
#[test]
fn test_guest2_exports_run_function() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance.get_typed_func::<(Option<String>,), (String,)>(&mut store, "run");
    assert!(run.is_ok(), "guest2 must export `run`");
}

/// Verifies that guest2 exports the `describe` function.
///
/// # Panics
///
/// Panics if the component fails to instantiate or `describe` is not found.
#[test]
fn test_guest2_exports_describe_function() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let describe = instance.get_typed_func::<(), (String,)>(&mut store, "describe");
    assert!(describe.is_ok(), "guest2 must export `describe`");
}

/// Verifies that guest1 does not export the `describe` function.
///
/// # Panics
///
/// Panics if `describe` is unexpectedly found in guest1.
#[test]
fn test_guest1_does_not_export_describe() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST1_WASM);
    let linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let describe = instance.get_typed_func::<(), (String,)>(&mut store, "describe");
    assert!(describe.is_err(), "guest1 must not export `describe`");
}

/// Verifies that guest1's `run` returns a string containing `guest1`.
///
/// # Panics
///
/// Panics if the returned string does not contain `guest1`.
#[test]
fn test_guest1_run_returns_expected_string() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST1_WASM);
    let result = call_run_no_args(&engine, &component);
    assert!(result.contains("guest1"), "result must contain 'guest1'");
}

/// Verifies that guest2's `run` returns a string containing `guest2`.
///
/// # Panics
///
/// Panics if the returned string does not contain `guest2`.
#[test]
fn test_guest2_run_returns_expected_string() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let result = call_run_with_name(&engine, &component, Some("Pulley"));
    assert!(result.contains("guest2"), "result must contain 'guest2'");
}

/// Verifies that guest2's `describe` returns the expected string.
///
/// # Panics
///
/// Panics if `describe` returns an unexpected value.
#[test]
fn test_guest2_describe_returns_expected_string() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let result = call_describe(&engine, &component);
    assert_eq!(result, "guest2 has an extra `describe` export");
}

/// Verifies that guest1 has no WASI imports.
///
/// # Panics
///
/// Panics if WASI-related imports are found in guest1.
#[test]
fn test_guest1_has_no_wasi_imports() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST1_WASM);
    let ty = component.component_type();
    let import_names: Vec<_> = ty
        .imports(&engine)
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(
        !import_names.iter().any(|n| n.contains("wasi")),
        "guest1 must not import WASI interfaces"
    );
}

/// Verifies that guest2 has no WASI imports.
///
/// # Panics
///
/// Panics if WASI-related imports are found in guest2.
#[test]
fn test_guest2_has_no_wasi_imports() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let ty = component.component_type();
    let import_names: Vec<_> = ty
        .imports(&engine)
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(
        !import_names.iter().any(|n| n.contains("wasi")),
        "guest2 must not import WASI interfaces"
    );
}

/// Verifies that guest2's `run` uses the default name when `None` is passed.
///
/// # Panics
///
/// Panics if the returned string does not contain `world`.
#[test]
fn test_guest2_run_default_name() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let result = call_run_with_name(&engine, &component, None);
    assert!(
        result.contains("world"),
        "result must contain default 'world'"
    );
}

/// Verifies that guest2's `run` uses the provided name when `Some` is passed.
///
/// # Panics
///
/// Panics if the returned string does not contain the provided name.
#[test]
fn test_guest2_run_custom_name() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let result = call_run_with_name(&engine, &component, Some("Pulley"));
    assert!(result.contains("Pulley"), "result must contain 'Pulley'");
}

/// Verifies that guest1's `run` returns the exact expected message.
///
/// # Panics
///
/// Panics if the returned string does not match.
#[test]
fn test_guest1_run_exact_message() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST1_WASM);
    let result = call_run_no_args(&engine, &component);
    assert_eq!(result, "guest1 run() called");
}

/// Verifies that guest2's `run` returns the exact greeting for a given name.
///
/// # Panics
///
/// Panics if the returned string does not match.
#[test]
fn test_guest2_run_exact_greeting() {
    let engine = create_engine();
    let component = compile_component(&engine, GUEST2_WASM);
    let result = call_run_with_name(&engine, &component, Some("Pulley"));
    assert_eq!(result, "guest2 run() called: hello, Pulley!");
}
