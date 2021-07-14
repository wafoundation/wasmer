use crate::export::Export;
use crate::exports::Exports;
use crate::externals::Extern;
use crate::module::Module;
use crate::store::Store;
// use crate::{HostEnvInitError, LinkError, RuntimeError};
use crate::resolver::Resolver;
use js_sys::WebAssembly;
use std::fmt;
use thiserror::Error;

/// A WebAssembly Instance is a stateful, executable
/// instance of a WebAssembly [`Module`].
///
/// Instance objects contain all the exported WebAssembly
/// functions, memories, tables and globals that allow
/// interacting with WebAssembly.
///
/// Spec: <https://webassembly.github.io/spec/core/exec/runtime.html#module-instances>
#[derive(Clone)]
pub struct Instance {
    instance: WebAssembly::Instance,
    module: Module,
    /// The exports for an instance.
    pub exports: Exports,
}

/// An error while instantiating a module.
///
/// This is not a common WebAssembly error, however
/// we need to differentiate from a `LinkError` (an error
/// that happens while linking, on instantiation), a
/// Trap that occurs when calling the WebAssembly module
/// start function, and an error when initializing the user's
/// host environments.
#[derive(Error, Debug)]
pub enum InstantiationError {
    /// A linking ocurred during instantiation.
    #[cfg_attr(feature = "std", error("Link error: {0}"))]
    Link(String),

    /// A runtime error occured while invoking the start function
    #[cfg_attr(feature = "std", error("Start error: {0}"))]
    Start(String),
    // /// Error occurred when initializing the host environment.
    // #[error(transparent)]
    // HostEnvInitialization(HostEnvInitError),
}

// impl From<wasmer_engine::InstantiationError> for InstantiationError {
//     fn from(other: wasmer_engine::InstantiationError) -> Self {
//         match other {
//             wasmer_engine::InstantiationError::Link(e) => Self::Link(e),
//             wasmer_engine::InstantiationError::Start(e) => Self::Start(e),
//         }
//     }
// }

// impl From<HostEnvInitError> for InstantiationError {
//     fn from(other: HostEnvInitError) -> Self {
//         Self::HostEnvInitialization(other)
//     }
// }

impl Instance {
    /// Creates a new `Instance` from a WebAssembly [`Module`] and a
    /// set of imports resolved by the [`Resolver`].
    ///
    /// The resolver can be anything that implements the [`Resolver`] trait,
    /// so you can plug custom resolution for the imports, if you wish not
    /// to use [`ImportObject`].
    ///
    /// The [`ImportObject`] is the easiest way to provide imports to the instance.
    ///
    /// [`ImportObject`]: crate::ImportObject
    ///
    /// ```
    /// # use wasmer::{imports, Store, Module, Global, Value, Instance};
    /// # fn main() -> anyhow::Result<()> {
    /// let store = Store::default();
    /// let module = Module::new(&store, "(module)")?;
    /// let imports = imports!{
    ///   "host" => {
    ///     "var" => Global::new(&store, Value::I32(2))
    ///   }
    /// };
    /// let instance = Instance::new(&module, &imports)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// The function can return [`InstantiationError`]s.
    ///
    /// Those are, as defined by the spec:
    ///  * Link errors that happen when plugging the imports into the instance
    ///  * Runtime errors that happen when running the module `start` function.
    pub fn new(module: &Module, resolver: &dyn Resolver) -> Result<Self, InstantiationError> {
        let store = module.store();
        let (instance, functions) = module.instantiate(resolver).unwrap();
        let instance_exports = instance.exports();
        let exports = module
            .exports()
            .map(|export_type| {
                let name = export_type.name();
                let extern_type = export_type.ty().clone();
                let js_export = js_sys::Reflect::get(&instance_exports, &name.into()).unwrap();
                let export: Export = (js_export, extern_type).into();
                let extern_ = Extern::from_vm_export(store, export);
                (name.to_string(), extern_)
            })
            .collect::<Exports>();

        let self_instance = Self {
            module: module.clone(),
            instance,
            exports,
        };
        for func in functions {
            func.init_envs(&self_instance).unwrap();
        }
        Ok(self_instance)
    }

    /// Gets the [`Module`] associated with this instance.
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Returns the [`Store`] where the `Instance` belongs.
    pub fn store(&self) -> &Store {
        self.module.store()
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Instance")
            .field("exports", &self.exports)
            .finish()
    }
}
