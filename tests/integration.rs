//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Integration Tests for Component-Model Guests on Pulley
//!
//! Validates that the compiled WASM guest components load correctly through
//! the Component Model, instantiate with WASI support, export the expected
//! functions (`run` and optionally `describe`), and produce correct output
//! when executed via the Pulley interpreter backend.

/// Component Model loader, linker, and resource table types.
use wasmtime::component::{Component, Linker, ResourceTable};
/// Wasmtime runtime core types.
use wasmtime::{Config, Engine, Store};
/// In-memory output pipe for capturing guest stdout in tests.
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
/// WASI context, view trait, and implementation for host state.
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

/// Filesystem path to the built guest1 component artifact.
const GUEST1_PATH: &str = "guest1/target/wasm32-wasip1/debug/guest1.wasm";

/// Filesystem path to the built guest2 component artifact.
const GUEST2_PATH: &str = "guest2/target/wasm32-wasip1/debug/guest2.wasm";

/// Host state for test component instantiation with WASI support.
struct TestHostState {
    /// WASI context containing stdio configuration for test execution.
    ctx: WasiCtx,
    /// Resource table required by component-model/WASI resource handles.
    table: ResourceTable,
}

impl WasiView for TestHostState {
    /// Returns mutable access to the WASI context and resource table.
    ///
    /// # Returns
    ///
    /// A `WasiCtxView` containing references to this state's `ctx` and `table`.
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

/// Creates a wasmtime engine configured for Component Model execution.
///
/// # Returns
///
/// A wasmtime `Engine` with component-model support enabled.
///
/// # Panics
///
/// Panics if engine creation fails.
fn create_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).expect("create component engine")
}

/// Loads a guest component artifact from disk.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine to compile with.
/// * `path` - Filesystem path to the component `.wasm` file.
///
/// # Returns
///
/// The compiled WASM `Component`.
///
/// # Panics
///
/// Panics if the component file cannot be read or compiled.
fn load_component(engine: &Engine, path: &str) -> Component {
    Component::from_file(engine, path).expect("load component")
}

/// Builds a fully configured test linker with WASI interfaces registered.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine to associate the linker with.
///
/// # Returns
///
/// A component `Linker` with all WASI sync interfaces registered.
///
/// # Panics
///
/// Panics if WASI interface registration fails.
fn build_test_linker(engine: &Engine) -> Linker<TestHostState> {
    let mut linker = Linker::<TestHostState>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("register WASI interfaces");
    linker
}

/// Creates a test store with inherited stdio.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine to create the store for.
///
/// # Returns
///
/// A `Store` containing a `TestHostState` with inherited stdio.
fn build_test_store(engine: &Engine) -> Store<TestHostState> {
    let state = TestHostState {
        ctx: WasiCtx::builder().inherit_stdio().build(),
        table: ResourceTable::new(),
    };
    Store::new(engine, state)
}

/// Creates a test store with captured stdout for output verification.
///
/// # Arguments
///
/// * `engine` - The wasmtime engine to create the store for.
///
/// # Returns
///
/// A tuple of the `Store` and the `MemoryOutputPipe` capturing stdout.
fn build_capture_store(engine: &Engine) -> (Store<TestHostState>, MemoryOutputPipe) {
    let stdout = MemoryOutputPipe::new(4096);
    let state = TestHostState {
        ctx: WasiCtx::builder().stdout(stdout.clone()).build(),
        table: ResourceTable::new(),
    };
    (Store::new(engine, state), stdout)
}

/// Verifies that the guest1 component binary loads without error.
///
/// # Panics
///
/// Panics if the guest1 component binary fails to compile.
#[test]
fn test_guest1_component_loads() {
    let engine = create_engine();
    let _component = load_component(&engine, GUEST1_PATH);
}

/// Verifies that the guest2 component binary loads without error.
///
/// # Panics
///
/// Panics if the guest2 component binary fails to compile.
#[test]
fn test_guest2_component_loads() {
    let engine = create_engine();
    let _component = load_component(&engine, GUEST2_PATH);
}

/// Verifies that guest1 instantiates and exports the `run` function.
///
/// # Panics
///
/// Panics if the component fails to instantiate or `run` is not found.
#[test]
fn test_guest1_exports_run_function() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST1_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let run = instance.get_typed_func::<(), ()>(&mut store, "run");
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
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance.get_typed_func::<(Option<String>,), ()>(&mut store, "run");
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
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
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
    let component = load_component(&engine, GUEST1_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let describe = instance.get_typed_func::<(), (String,)>(&mut store, "describe");
    assert!(describe.is_err(), "guest1 must not export `describe`");
}

/// Verifies that guest1's `run` executes without error.
///
/// # Panics
///
/// Panics if `run` traps or returns an error.
#[test]
fn test_guest1_run_executes_successfully() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST1_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let run = instance
        .get_typed_func::<(), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, ()).expect("execute run");
}

/// Verifies that guest2's `run` executes without error.
///
/// # Panics
///
/// Panics if `run` traps or returns an error.
#[test]
fn test_guest2_run_executes_successfully() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance
        .get_typed_func::<(Option<String>,), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, (Some("Pulley".to_string()),))
        .expect("execute run");
}

/// Verifies that guest2's `describe` returns the expected string.
///
/// # Panics
///
/// Panics if `describe` returns an unexpected value.
#[test]
fn test_guest2_describe_returns_expected_string() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let mut store = build_test_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let describe = instance
        .get_typed_func::<(), (String,)>(&mut store, "describe")
        .expect("get describe");
    let (message,) = describe.call(&mut store, ()).expect("execute describe");
    assert_eq!(message, "guest2 has an extra `describe` export");
}

/// Verifies that guest1 imports WASI interfaces.
///
/// # Panics
///
/// Panics if no WASI-related imports are found.
#[test]
fn test_guest1_imports_wasi() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST1_PATH);
    let ty = component.component_type();
    let import_names: Vec<_> = ty
        .imports(&engine)
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(!import_names.is_empty(), "guest1 must have imports");
    assert!(
        import_names.iter().any(|n| n.contains("wasi")),
        "guest1 must import WASI interfaces"
    );
}

/// Verifies that guest2 imports WASI interfaces.
///
/// # Panics
///
/// Panics if no WASI-related imports are found.
#[test]
fn test_guest2_imports_wasi() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let ty = component.component_type();
    let import_names: Vec<_> = ty
        .imports(&engine)
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(!import_names.is_empty(), "guest2 must have imports");
    assert!(
        import_names.iter().any(|n| n.contains("wasi")),
        "guest2 must import WASI interfaces"
    );
}

/// Verifies that guest1's `run` produces output containing `guest1`.
///
/// # Panics
///
/// Panics if no output is captured or `guest1` is not in stdout.
#[test]
fn test_guest1_run_produces_output() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST1_PATH);
    let linker = build_test_linker(&engine);
    let (mut store, stdout) = build_capture_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest1");
    let run = instance
        .get_typed_func::<(), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, ()).expect("execute run");
    drop(store);
    let output = String::from_utf8(stdout.contents().to_vec()).expect("valid UTF-8");
    assert!(output.contains("guest1"), "stdout must contain 'guest1'");
}

/// Verifies that guest2's `run` produces output containing `guest2`.
///
/// # Panics
///
/// Panics if no output is captured or `guest2` is not in stdout.
#[test]
fn test_guest2_run_produces_output() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let (mut store, stdout) = build_capture_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance
        .get_typed_func::<(Option<String>,), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, (Some("Pulley".to_string()),))
        .expect("execute run");
    drop(store);
    let output = String::from_utf8(stdout.contents().to_vec()).expect("valid UTF-8");
    assert!(output.contains("guest2"), "stdout must contain 'guest2'");
}

/// Verifies that guest2's `run` uses the default name when `None` is passed.
///
/// # Panics
///
/// Panics if stdout does not contain the default `"world"` greeting.
#[test]
fn test_guest2_run_default_name() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let (mut store, stdout) = build_capture_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance
        .get_typed_func::<(Option<String>,), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, (None,)).expect("execute run");
    drop(store);
    let output = String::from_utf8(stdout.contents().to_vec()).expect("valid UTF-8");
    assert!(
        output.contains("world"),
        "stdout must contain default 'world'"
    );
}

/// Verifies that guest2's `run` uses the provided name when `Some` is passed.
///
/// # Panics
///
/// Panics if stdout does not contain the provided name.
#[test]
fn test_guest2_run_custom_name() {
    let engine = create_engine();
    let component = load_component(&engine, GUEST2_PATH);
    let linker = build_test_linker(&engine);
    let (mut store, stdout) = build_capture_store(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiate guest2");
    let run = instance
        .get_typed_func::<(Option<String>,), ()>(&mut store, "run")
        .expect("get run");
    run.call(&mut store, (Some("Pulley".to_string()),))
        .expect("execute run");
    drop(store);
    let output = String::from_utf8(stdout.contents().to_vec()).expect("valid UTF-8");
    assert!(output.contains("Pulley"), "stdout must contain 'Pulley'");
}
