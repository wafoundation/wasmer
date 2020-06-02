# Wasmer Compiler

This crate is the base for Compiler implementations.

It performs the translation from a Wasm module into a basic ModuleInfo,
but leaves the Wasm function bytecode translation to the compiler implementor.

### Acknowledgments

This project borrowed some of the code strucutre from the [cranelift-wasm](https://crates.io/crates/cranelift-wasm), however it's been adapted to not depend on any specific IR and be abstract of any compiler.

Please check [Wasmer ATTRIBUTIONS](https://github.com/wasmerio/wasmer/blob/master/ATTRIBUTIONS.md) to further see licenses and other attributions of the project. 