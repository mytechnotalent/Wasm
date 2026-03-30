//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Guest1 WebAssembly Component
//!
//! Implements the `example` world generated from `guest1/wit/world.wit`.
//! Exports a single `run` function used by the host.

/// Generated bindings for the guest1 WIT world.
#[allow(warnings)]
mod bindings;

/// Guest trait generated from guest1's WIT world.
use bindings::Guest;

/// Concrete guest1 component implementation exported to the host.
struct Component;

impl Guest for Component {
    /// Entry point exported by the component world.
    ///
    /// # Returns
    ///
    /// Returns `()` after emitting a diagnostic message to stdout.
    fn run() {
        println!("guest1 run() called");
    }
}

// Exports `Component` as the guest implementation for generated bindings.
bindings::export!(Component with_types_in bindings);
