//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Component-Model Host Running on Pulley
//!
//! Loads WebAssembly Component Model guest artifacts produced by
//! `cargo-component`, instantiates them with WASI support, and executes
//! exported functions via the Pulley interpreter backend. WIT contracts
//! for guests are in `guest1/wit/world.wit` and `guest2/wit/world.wit`.

/// Component Model loader, linker, and resource table types.
use wasmtime::component::{Component, Linker, ResourceTable};
/// Wasmtime runtime core types.
use wasmtime::{Config, Engine, Result, Store};
/// WASI context, view trait, and implementation for host state.
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

/// Default name passed to guests that accept an `Option<String>` parameter.
const DEFAULT_GUEST_NAME: &str = "Pulley";

/// Pulley target selected for this host architecture.
///
/// This value is passed to `Config::target` so Wasmtime compiles and executes
/// for the Pulley backend instead of selecting a native host target.
const PULLEY_TARGET: &str = "pulley64";

/// Paths to built Rust guest components loaded by the host.
///
/// Each path points to a component artifact produced by `cargo component build`
/// for an individual guest package.
const COMPONENT_PATHS: [&str; 2] = [
    "guest1/target/wasm32-wasip1/debug/guest1.wasm",
    "guest2/target/wasm32-wasip1/debug/guest2.wasm",
];

/// Host state for component instantiation with WASI support.
struct HostState {
    /// WASI context containing stdio configuration and runtime capabilities.
    ctx: WasiCtx,
    /// Resource table required by component-model/WASI resource handles.
    table: ResourceTable,
}

impl WasiView for HostState {
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

/// Builds a Wasmtime engine configured for Component Model + Pulley.
///
/// # Returns
///
/// A configured `Engine` that enables component-model support and targets
/// `pulley64`.
///
/// # Errors
///
/// Returns an error if the Pulley target cannot be set or if engine creation
/// fails.
fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.target(PULLEY_TARGET)?;
    Engine::new(&config)
}

/// Creates a component linker with synchronous WASI imports registered.
///
/// # Arguments
///
/// * `engine` - Engine used to create the linker and bind host interfaces.
///
/// # Returns
///
/// A `Linker<HostState>` ready to instantiate components that import WASI.
///
/// # Errors
///
/// Returns an error if WASI interfaces cannot be added to the linker.
fn build_linker(engine: &Engine) -> Result<Linker<HostState>> {
    let mut linker = Linker::<HostState>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    Ok(linker)
}

/// Creates store state with inherited stdio for guest output.
///
/// # Arguments
///
/// * `engine` - Engine used to construct the store.
///
/// # Returns
///
/// A `Store<HostState>` initialized with WASI stdio inheritance and an empty
/// resource table.
fn build_store(engine: &Engine) -> Store<HostState> {
    let state = HostState {
        ctx: WasiCtx::builder().inherit_stdio().build(),
        table: ResourceTable::new(),
    };
    Store::new(engine, state)
}

/// Loads one component artifact, instantiates it, and executes exported calls.
///
/// This function first tries to invoke `run` with an `Option<String>` parameter.
/// If the export does not accept a parameter, it falls back to calling `run`
/// with no arguments. It also checks for an optional `describe` export, and
/// if present, invokes it and prints the returned message.
///
/// # Arguments
///
/// * `engine` - Engine used to compile and instantiate the component.
/// * `path` - Filesystem path to the component artifact (`.wasm`).
/// * `name` - Name string passed to guests that accept an `Option<String>` parameter.
///
/// # Returns
///
/// `Ok(())` if loading, instantiation, and all invoked exports succeed.
///
/// # Errors
///
/// Returns an error if:
///
/// * The component file cannot be read or compiled.
/// * WASI linker setup fails.
/// * Instantiation fails due to missing or incompatible imports.
/// * The required `run` export is missing or has an incompatible type.
/// * A called export traps or returns an invocation error.
fn run_component(engine: &Engine, path: &str, name: &str) -> Result<()> {
    let component = Component::from_file(engine, path)?;
    let linker = build_linker(engine)?;
    let mut store = build_store(engine);
    let instance = linker.instantiate(&mut store, &component)?;
    if let Ok(run) = instance.get_typed_func::<(Option<String>,), ()>(&mut store, "run") {
        run.call(&mut store, (Some(name.to_string()),))?;
    } else {
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
    }
    if let Ok(describe) = instance.get_typed_func::<(), (String,)>(&mut store, "describe") {
        let (message,) = describe.call(&mut store, ())?;
        println!("describe: {}", message);
    }
    Ok(())
}

/// Runs the full host flow for all configured component artifacts.
///
/// Reads an optional name from the first CLI argument (defaults to
/// `DEFAULT_GUEST_NAME`). The sequence is:
///
/// 1. Parse CLI arguments for an optional guest name.
/// 2. Create an engine configured for component-model execution on Pulley.
/// 3. Iterate over `COMPONENT_PATHS`.
/// 4. For each artifact, load, instantiate, and invoke exports.
///
/// # Returns
///
/// `Ok(())` when all configured components execute successfully.
///
/// # Errors
///
/// Returns the first error encountered while configuring the engine or running
/// any component.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).map_or(DEFAULT_GUEST_NAME, |s| s.as_str());
    println!("Building Pulley component engine...");
    let engine = build_engine()?;
    for path in COMPONENT_PATHS {
        println!("Compiling component from {}...", path);
        println!("Instantiating and calling run...");
        run_component(&engine, path, name)?;
    }
    println!("Done.");
    Ok(())
}
