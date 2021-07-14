use crate::lib::std::string::String;
#[cfg(feature = "std")]
use thiserror::Error;

// Compilation Errors
//
// If `std` feature is enable, we can't use `thiserror` until
// https://github.com/dtolnay/thiserror/pull/64 is merged.

/// The WebAssembly.CompileError object indicates an error during
/// WebAssembly decoding or validation.
///
/// This is based on the [Wasm Compile Error][compile-error] API.
///
/// [compiler-error]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WebAssembly/CompileError
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum CompileError {
    /// A Wasm translation error occured.
    #[cfg_attr(feature = "std", error("WebAssembly translation error: {0}"))]
    Wasm(WasmError),

    /// A compilation error occured.
    #[cfg_attr(feature = "std", error("Compilation error: {0}"))]
    Codegen(String),

    /// The module did not pass validation.
    #[cfg_attr(feature = "std", error("Validation error: {0}"))]
    Validate(String),

    /// The compiler doesn't support a Wasm feature
    #[cfg_attr(feature = "std", error("Feature {0} is not yet supported"))]
    UnsupportedFeature(String),

    /// The compiler cannot compile for the given target.
    /// This can refer to the OS, the chipset or any other aspect of the target system.
    #[cfg_attr(feature = "std", error("The target {0} is not yet supported (see https://docs.wasmer.io/ecosystem/wasmer/wasmer-features)"))]
    UnsupportedTarget(String),

    /// Insufficient resources available for execution.
    #[cfg_attr(feature = "std", error("Insufficient resources: {0}"))]
    Resource(String),
}

impl From<WasmError> for CompileError {
    fn from(original: WasmError) -> Self {
        Self::Wasm(original)
    }
}

/// A WebAssembly translation error.
///
/// When a WebAssembly function can't be translated, one of these error codes will be returned
/// to describe the failure.
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum WasmError {
    /// The input WebAssembly code is invalid.
    ///
    /// This error code is used by a WebAssembly translator when it encounters invalid WebAssembly
    /// code. This should never happen for validated WebAssembly code.
    #[cfg_attr(
        feature = "std",
        error("Invalid input WebAssembly code at offset {offset}: {message}")
    )]
    InvalidWebAssembly {
        /// A string describing the validation error.
        message: String,
        /// The bytecode offset where the error occurred.
        offset: usize,
    },

    /// A feature used by the WebAssembly code is not supported by the embedding environment.
    ///
    /// Embedding environments may have their own limitations and feature restrictions.
    #[cfg_attr(feature = "std", error("Unsupported feature: {0}"))]
    Unsupported(String),

    /// A generic error.
    #[cfg_attr(feature = "std", error("{0}"))]
    Generic(String),
}
