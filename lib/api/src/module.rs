use crate::store::Store;
use crate::types::{ExportType, ImportType};
use std::io;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use wasmer_compiler::{CompileError, WasmError};
use wasmer_jit::{CompiledModule, DeserializeError, SerializeError};

#[derive(Error, Debug)]
pub enum IoCompileError {
    /// An IO error
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A compilation error
    #[error(transparent)]
    Compile(#[from] CompileError),
}

/// A WebAssembly Module contains stateless WebAssembly
/// code that has already been compiled and can be instantiated
/// multiple times.
///
/// ## Cloning a module
///
/// Cloning a moudle is cheap: it does a shallow copy of the compiled
/// contents rather than a deep copy.
#[derive(Clone)]
pub struct Module {
    store: Store,
    compiled: Arc<CompiledModule>,
}

impl Module {
    /// Creates a new WebAssembly Module given the configuration
    /// in the store.
    ///
    /// If the provided bytes are not WebAssembly-like (start with `b"\0asm"`),
    /// and the "wat" feature is enabled for this crate, this function will try to
    /// to convert the bytes assuming they correspond to the WebAssembly text
    /// format.
    ///
    /// ## Security
    ///
    /// Before the code is compiled, it will be validated using the store
    /// features.
    ///
    /// ## Errors
    ///
    /// Creating a WebAssembly module from bytecode can result in a
    /// [`CompileError`] since this operation requires to transorm the Wasm
    /// bytecode into code the machine can easily execute (normally through a JIT).
    ///
    /// ## Example
    ///
    /// Reading from a WAT file.
    ///
    /// ```
    /// let wat = "(module)";
    /// let module = Module::new(&store, wat)?;
    /// ```
    ///
    /// Reading from bytes:
    ///
    /// ```
    /// let bytes: Vec<u8> = vec![];
    /// let module = Module::new(&store, bytes)?;
    /// ```
    pub fn new(store: &Store, bytes: impl AsRef<[u8]>) -> Result<Module, CompileError> {
        // We try to parse it with WAT: it will be a no-op on
        // wasm files.
        if bytes.as_ref().starts_with(b"\0asm") {
            return Module::from_binary(store, bytes.as_ref());
        }

        #[cfg(feature = "wat")]
        {
            let bytes = wat::parse_bytes(bytes.as_ref())
                .map_err(|e| CompileError::Wasm(WasmError::Generic(format!("{}", e))))?;
            // We can assume the binary is valid WebAssembly if returned
            // without errors from from wat. However, by skipping validation
            // we are not checking if it's using WebAssembly features not enabled
            // in the store.
            // This is a good tradeoff, as we can assume the "wat" feature is only
            // going to be used in development mode.
            return unsafe { Module::from_binary_unchecked(store, bytes.as_ref()) };
        }

        Err(CompileError::Validate(
            "The module is not a valid WebAssembly file.".to_string(),
        ))
    }

    pub fn from_file(store: &Store, file: impl AsRef<Path>) -> Result<Module, IoCompileError> {
        let file_ref = file.as_ref();
        let canonical = file_ref.canonicalize()?;
        let wasm_bytes = std::fs::read(file_ref)?;
        let mut module = Module::new(store, &wasm_bytes)?;
        // Set the module name to the absolute path of the filename.
        // This is useful for debugging the stack traces.
        let filename = canonical.as_path().to_str().unwrap();
        module.set_name(filename);
        Ok(module)
    }

    ///  Creates a new WebAssembly module from a binary.
    ///
    /// Opposed to [`Module::new`], this function is not compatible with
    /// the WebAssembly text format (if the "wat" feature is enabled for
    /// this crate).
    pub fn from_binary(store: &Store, binary: &[u8]) -> Result<Module, CompileError> {
        Module::validate(store, binary)?;
        unsafe { Module::from_binary_unchecked(store, binary) }
    }

    /// Creates a new WebAssembly module skipping any kind of validation.
    ///
    /// This can speed up compilation time a bit, but it should be only used
    /// in environments where the WebAssembly modules are trusted and validated
    /// beforehand.
    pub unsafe fn from_binary_unchecked(
        store: &Store,
        binary: &[u8],
    ) -> Result<Module, CompileError> {
        let module = Module::compile(store, binary)?;
        Ok(module)
    }

    /// Validates a new WebAssembly Module given the configuration
    /// in the Store.
    ///
    /// This validation is normally pretty fast and checks the enabled
    /// WebAssembly features in the Store Engine to assure deterministic
    /// validation of the Module.
    pub fn validate(store: &Store, binary: &[u8]) -> Result<(), CompileError> {
        store.engine().validate(binary)
    }

    fn compile(store: &Store, binary: &[u8]) -> Result<Self, CompileError> {
        let compiled = store.engine().compile(binary)?;
        Ok(Self::from_compiled_module(store, compiled))
    }

    /// Serializes a module into it a propietary serializable format,
    /// so it can be used later by [`Module::deserialize`].
    ///
    /// # Usage
    ///
    /// ```ignore
    /// # use wasmer::*;
    /// # let store = Store::default();
    /// # let module = Module::from_file(&store, "path/to/foo.wasm")?;
    /// let serialized = module.serialize()?;
    /// ```
    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        self.store.engine().serialize(self.compiled_module())
    }

    /// Deserializes a a serialized Module binary into a `Module`.
    /// > Note: the module has to be serialized before with the `serialize` method.
    ///
    /// # Unsafety
    ///
    /// This function is inherently `unsafe` as the provided bytes:
    /// 1. Are going to be deserialized directly into Rust objects.
    /// 2. Contains the function assembly bodies and, if intercepted,
    ///    a malicious actor could inject code into executable
    ///    memory.
    ///
    /// And as such, the `deserialize` method is unsafe.
    ///
    /// # Usage
    ///
    /// ```ignore
    /// # use wasmer::*;
    /// # let store = Store::default();
    /// let module = Module::deserialize(&store, serialized_data)?;
    /// ```
    pub unsafe fn deserialize(store: &Store, bytes: &[u8]) -> Result<Self, DeserializeError> {
        let compiled = store.engine().deserialize(bytes)?;
        Ok(Self::from_compiled_module(store, compiled))
    }

    fn from_compiled_module(store: &Store, compiled: CompiledModule) -> Self {
        Module {
            store: store.clone(),
            compiled: Arc::new(compiled),
        }
    }

    pub(crate) fn compiled_module(&self) -> &CompiledModule {
        &self.compiled
    }

    /// Returns the name of the current module.
    ///
    /// This name is normally set in the WebAssembly bytecode by some
    /// compilers, but can be also overwritten using the [`Module::set_name`] method.
    ///
    /// # Example
    ///
    /// ```
    /// let wat = "(module $moduleName)";
    /// let module = Module::new(&store, wat)?;
    /// assert_eq!(module.name(), Some("moduleName"));
    /// ```
    pub fn name(&self) -> Option<&str> {
        self.compiled.module().name.as_deref()
    }

    /// Sets the name of the current module.
    ///
    /// This is normally useful for stacktraces and debugging.
    ///
    /// # Example
    ///
    /// ```
    /// let wat = "(module)";
    /// let module = Module::new(&store, wat)?;
    /// assert_eq!(module.name(), None);
    /// module.set_name("foo");
    /// assert_eq!(module.name(), Some("foo"));
    /// ```
    pub fn set_name(&mut self, name: &str) {
        let compiled = Arc::get_mut(&mut self.compiled).unwrap();
        Arc::get_mut(compiled.module_mut()).unwrap().name = Some(name.to_string());
    }

    /// Returns an iterator over the imported types in the Module.
    ///
    /// The order of the imports is guaranteed to be the same as in the
    /// WebAssembly bytecode.
    ///
    /// # Example
    ///
    /// ```
    /// # let store = Store::default();
    /// let wat = r#"(module
    ///     (import "host" "func1" (func))
    ///     (import "host" "func2" (func))
    /// )"#;
    /// let module = Module::new(&store, wat)?;
    /// for import in module.imports() {
    ///     assert_eq!(import.module(), "host");
    ///     assert!(import.name().contains("func"));
    ///     import.ty();
    /// }
    /// ```
    pub fn imports<'a>(&'a self) -> impl Iterator<Item = ImportType> + 'a {
        self.compiled.module_ref().imports()
    }

    /// Returns an iterator over the exported types in the Module.
    ///
    /// The order of the exports is guaranteed to be the same as in the
    /// WebAssembly bytecode.
    ///
    /// # Example
    ///
    /// ```
    /// # let store = Store::default();
    /// let wat = r#"(module
    ///     (func (export "namedfunc"))
    ///     (memory (export "namedmemory") 1)
    /// )"#;
    /// let module = Module::new(&store, wat)?;
    /// for import in module.exports() {
    ///     assert_eq!(export.name().contains("named"));
    ///     export.ty();
    /// }
    /// ```
    pub fn exports<'a>(&'a self) -> impl Iterator<Item = ExportType> + 'a {
        self.compiled.module_ref().exports()
    }

    pub fn store(&self) -> &Store {
        &self.store
    }
}