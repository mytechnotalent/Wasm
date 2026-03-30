//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Guest2 WebAssembly Component
//!
//! Implements the `example` world generated from `guest2/wit/world.wit`.
//! Exports `run` (with an optional `name` parameter) and `describe` to
//! demonstrate a guest API that is intentionally different from guest1.

/// Generated bindings for the guest2 WIT world.
#[allow(warnings)]
mod bindings;

/// Guest trait generated from guest2's WIT world.
use bindings::Guest;

/// Default name used when no parameter is provided to `run`.
const DEFAULT_NAME: &str = "world";

/// Concrete guest2 component implementation exported to the host.
struct Component;

impl Guest for Component {
    /// Entry point exported by the component world.
    ///
    /// # Arguments
    ///
    /// * `name` - Optional greeting name; defaults to `"world"` when `None`.
    fn run(name: Option<String>) {
        let name = name.as_deref().unwrap_or(DEFAULT_NAME);
        println!("guest2 run() called: hello, {name}!");
    }

    /// Extra export to make guest2's WIT/API distinct from guest1.
    ///
    /// # Returns
    ///
    /// Returns a short description string consumed by the host when present.
    fn describe() -> String {
        "guest2 has an extra `describe` export".to_string()
    }
}

// Exports `Component` as the guest implementation for generated bindings.
bindings::export!(Component with_types_in bindings);
