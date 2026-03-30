//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Guest2 WebAssembly Component
//!
//! Implements the `guest2-world` generated from `guest2/wit/world.wit`.
//! Exports `run` (with an optional `name` parameter) and `describe` to
//! demonstrate a guest API that is intentionally different from guest1.

#![no_std]

// Enable the global allocator for heap-backed collections.
extern crate alloc;

use alloc::format; // String formatting macro for no_std.
use alloc::string::String; // Owned string type for no_std.

/// Global heap allocator required by the canonical ABI's `cabi_realloc`.
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// Generate guest-side bindings for the `guest2-world` WIT world.
wit_bindgen::generate!({
    world: "guest2-world",
    path: "wit",
});

/// Default name used when no parameter is provided to `run`.
const DEFAULT_NAME: &str = "world";

/// Concrete guest2 component implementation exported to the host.
struct Component;

// Register `Component` as the component's exported implementation.
export!(Component);

impl Guest for Component {
    /// Entry point exported by the component world.
    ///
    /// # Arguments
    ///
    /// * `name` - Optional greeting name; defaults to `"world"` when `None`.
    ///
    /// # Returns
    ///
    /// A greeting string.
    fn run(name: Option<String>) -> String {
        let name = name.as_deref().unwrap_or(DEFAULT_NAME);
        format!("guest2 run() called: hello, {name}!")
    }

    /// Extra export to make guest2's WIT/API distinct from guest1.
    ///
    /// # Returns
    ///
    /// Returns a short description string consumed by the host when present.
    fn describe() -> String {
        String::from("guest2 has an extra `describe` export")
    }
}
