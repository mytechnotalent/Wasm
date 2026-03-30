//! SPDX-License-Identifier: MIT
//!
//! Copyright (c) 2026 Kevin Thomas
//!
//! # Guest1 WebAssembly Component
//!
//! Implements the `guest1-world` generated from `guest1/wit/world.wit`.
//! Exports a single `run` function used by the host.

#![no_std]

// Enable the global allocator for heap-backed collections.
extern crate alloc;

use alloc::string::String; // Owned string type for no_std.

/// Global heap allocator required by the canonical ABI's `cabi_realloc`.
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// Generate guest-side bindings for the `guest1-world` WIT world.
wit_bindgen::generate!({
    world: "guest1-world",
    path: "wit",
});

/// Concrete guest1 component implementation exported to the host.
struct Component;

// Register `Component` as the component's exported implementation.
export!(Component);

impl Guest for Component {
    /// Entry point exported by the component world.
    ///
    /// # Returns
    ///
    /// Returns a diagnostic message string.
    fn run() -> String {
        String::from("guest1 run() called")
    }
}
