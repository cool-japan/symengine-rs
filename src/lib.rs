//! Symbolic computation in Rust, powered by [SymEngine](https://github.com/symengine/symengine)
//!
//! This crate provides Rust bindings to SymEngine. SymEngine is a fast symbolic manipulation
//! library, written in C++.

pub mod expr;
pub mod map;

pub use expr::Expression;
pub use map::ExprMap;

// Reexport symengine_sys to ensure the same version is always used
pub use symengine_sys;