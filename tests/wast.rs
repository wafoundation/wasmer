#![cfg(all(feature = "compiler", feature = "engine"))]

use std::path::Path;
use std::sync::Arc;
use test_utils::get_compiler_config_from_str;
use wasmer::{Features, Store, Tunables};
#[cfg(feature = "jit")]
use wasmer_engine_jit::JITEngine;
use wasmer_wast::Wast;

// The generated tests (from build.rs) look like:
// #[cfg(test)]
// mod singlepass {
//     mod spec {
//         #[test]
//         fn address() -> anyhow::Result<()> {
//             crate::run_wast("tests/spectests/address.wast", "singlepass")
//         }
//     }
// }
include!(concat!(env!("OUT_DIR"), "/generated_spectests.rs"));

fn run_wast(wast_path: &str, compiler: &str) -> anyhow::Result<()> {
    println!(
        "Running wast `{}` with the {} compiler",
        wast_path, compiler
    );
    let try_nan_canonicalization = wast_path.contains("nan-canonicalization");
    let mut features = Features::default();
    if wast_path.contains("bulk-memory") {
        features.bulk_memory(true);
    }
    let compiler_config =
        get_compiler_config_from_str(compiler, try_nan_canonicalization, features);
    let tunables = Tunables::for_target(compiler_config.target().triple());
    let store = Store::new(Arc::new(JITEngine::new(&*compiler_config, tunables)));
    let mut wast = Wast::new_with_spectest(store);
    if compiler == "singlepass" || compiler == "llvm" {
        // We don't support multivalue yet in singlepass or llvm
        wast.allow_instantiation_failures(&[
            "Validation error: invalid result arity: func type returns multiple values",
            "Validation error: blocks, loops, and ifs accept no parameters when multi-value is not enabled"
        ]);
    }
    wast.fail_fast = false;
    let path = Path::new(wast_path);
    wast.run_file(path)
}