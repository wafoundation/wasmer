use crate::common::get_cache_dir;
#[cfg(feature = "debug")]
use crate::logging;
use crate::store::{CompilerType, EngineType, StoreOptions};
use crate::suggestions::suggest_function_exports;
use crate::warning;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::str::FromStr;
use wasmer::*;
#[cfg(feature = "cache")]
use wasmer_cache::{Cache, FileSystemCache, Hash};

use structopt::StructOpt;

#[cfg(feature = "wasi")]
mod wasi;

#[cfg(feature = "wasi")]
use wasi::Wasi;

#[derive(Debug, StructOpt, Clone)]
/// The options for the `wasmer run` subcommand
pub struct Run {
    /// Disable the cache
    #[structopt(long = "disable-cache")]
    disable_cache: bool,

    /// File to run
    #[structopt(name = "FILE", parse(from_os_str))]
    path: PathBuf,

    /// Invoke a specified function
    #[structopt(long = "invoke", short = "i")]
    invoke: Option<String>,

    /// The command name is a string that will override the first argument passed
    /// to the wasm program. This is used in wapm to provide nicer output in
    /// help commands and error messages of the running wasm program
    #[structopt(long = "command-name", hidden = true)]
    command_name: Option<String>,

    /// A prehashed string, used to speed up start times by avoiding hashing the
    /// wasm module. If the specified hash is not found, Wasmer will hash the module
    /// as if no `cache-key` argument was passed.
    #[structopt(long = "cache-key", hidden = true)]
    cache_key: Option<String>,

    #[structopt(flatten)]
    store: StoreOptions,

    // TODO: refactor WASI structure to allow shared options with Emscripten
    #[cfg(feature = "wasi")]
    #[structopt(flatten)]
    wasi: Wasi,

    /// Enable non-standard experimental IO devices
    #[cfg(feature = "io-devices")]
    #[structopt(long = "enable-io-devices")]
    enable_experimental_io_devices: bool,

    /// Enable debug output
    #[cfg(feature = "debug")]
    #[structopt(long = "debug", short = "d")]
    debug: bool,

    /// Application arguments
    #[structopt(name = "--", multiple = true)]
    args: Vec<String>,
}

impl Run {
    /// Execute the run command
    pub fn execute(&self) -> Result<()> {
        #[cfg(feature = "debug")]
        if self.debug {
            logging::set_up_logging().unwrap();
        }
        self.inner_execute().with_context(|| {
            format!(
                "failed to run `{}`{}",
                self.path.display(),
                if CompilerType::enabled().is_empty() {
                    " (no compilers enabled)"
                } else {
                    ""
                }
            )
        })
    }

    fn inner_execute(&self) -> Result<()> {
        let module = self.get_module()?;
        // Do we want to invoke a function?
        if let Some(ref invoke) = self.invoke {
            let imports = imports! {};
            let instance = Instance::new(&module, &imports)?;
            let result = self.invoke_function(&instance, &invoke, &self.args)?;
            println!(
                "{}",
                result
                    .iter()
                    .map(|val| val.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
            return Ok(());
        }
        #[cfg(feature = "emscripten")]
        {
            use wasmer_emscripten::{
                generate_emscripten_env, is_emscripten_module, run_emscripten_instance, EmEnv,
                EmscriptenGlobals,
            };
            // TODO: refactor this
            if is_emscripten_module(&module) {
                let mut emscripten_globals = EmscriptenGlobals::new(module.store(), &module)
                    .map_err(|e| anyhow!("{}", e))?;
                let mut em_env = EmEnv::new(&emscripten_globals.data, Default::default());
                let import_object =
                    generate_emscripten_env(module.store(), &mut emscripten_globals, &mut em_env);
                let mut instance = match Instance::new(&module, &import_object) {
                    Ok(instance) => instance,
                    Err(e) => {
                        let err: Result<(), _> = Err(e);
                        #[cfg(feature = "wasi")]
                        {
                            if Wasi::has_wasi_imports(&module) {
                                return err.with_context(|| "This module has both Emscripten and WASI imports. Wasmer does not currently support Emscripten modules using WASI imports.");
                            }
                        }
                        return err.with_context(|| "Can't instantiate emscripten module");
                    }
                };

                run_emscripten_instance(
                    &mut instance,
                    &mut em_env,
                    &mut emscripten_globals,
                    if let Some(cn) = &self.command_name {
                        cn
                    } else {
                        self.path.to_str().unwrap()
                    },
                    self.args.iter().map(|arg| arg.as_str()).collect(),
                    None, //run.em_entrypoint.clone(),
                )?;
                return Ok(());
            }
        }

        // If WASI is enabled, try to execute it with it
        #[cfg(feature = "wasi")]
        {
            use std::collections::BTreeSet;
            use wasmer_wasi::WasiVersion;

            let wasi_versions = Wasi::get_versions(&module);
            if !wasi_versions.is_empty() {
                if wasi_versions.len() >= 2 {
                    let get_version_list = |versions: &BTreeSet<WasiVersion>| -> String {
                        versions
                            .iter()
                            .map(|v| format!("`{}`", v.get_namespace_str()))
                            .collect::<Vec<String>>()
                            .join(", ")
                    };
                    if self.wasi.deny_multiple_wasi_versions {
                        let version_list = get_version_list(&wasi_versions);
                        bail!("Found more than 1 WASI version in this module ({}) and `--deny-multiple-wasi-versions` is enabled.", version_list);
                    } else if !self.wasi.allow_multiple_wasi_versions {
                        let version_list = get_version_list(&wasi_versions);
                        warning!("Found more than 1 WASI version in this module ({}). If this is intentional, pass `--allow-multiple-wasi-versions` to suppress this warning.", version_list);
                    }
                }

                let program_name = self
                    .command_name
                    .clone()
                    .or_else(|| {
                        self.path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                    })
                    .unwrap_or_default();
                return self
                    .wasi
                    .execute(module, program_name, self.args.clone())
                    .with_context(|| "WASI execution failed");
            }
        }

        // Try to instantiate the wasm file, with no provided imports
        let imports = imports! {};
        let instance = Instance::new(&module, &imports)?;
        let start: Function = self.try_find_function(&instance, "_start", &[])?;
        start.call(&[])?;

        Ok(())
    }

    fn get_module(&self) -> Result<Module> {
        let contents = std::fs::read(self.path.clone())?;
        #[cfg(feature = "dylib")]
        {
            if wasmer_engine_dylib::DylibArtifact::is_deserializable(&contents) {
                let engine = wasmer_engine_dylib::Dylib::headless().engine();
                let store = Store::new(&engine);
                let module = unsafe { Module::deserialize_from_file(&store, &self.path)? };
                return Ok(module);
            }
        }
        #[cfg(feature = "universal")]
        {
            if wasmer_engine_universal::UniversalArtifact::is_deserializable(&contents) {
                let engine = wasmer_engine_universal::Universal::headless().engine();
                let store = Store::new(&engine);
                let module = unsafe { Module::deserialize_from_file(&store, &self.path)? };
                return Ok(module);
            }
        }
        let (store, engine_type, compiler_type) = self.store.get_store()?;
        #[cfg(feature = "cache")]
        let module_result: Result<Module> = if !self.disable_cache && contents.len() > 0x1000 {
            self.get_module_from_cache(&store, &contents, &engine_type, &compiler_type)
        } else {
            Module::new(&store, &contents).map_err(|e| e.into())
        };
        #[cfg(not(feature = "cache"))]
        let module_result = Module::new(&store, &contents);

        let mut module = module_result.with_context(|| {
            format!(
                "module instantiation failed (engine: {}, compiler: {})",
                engine_type.to_string(),
                compiler_type.to_string()
            )
        })?;
        // We set the name outside the cache, to make sure we dont cache the name
        module.set_name(&self.path.file_name().unwrap_or_default().to_string_lossy());

        Ok(module)
    }

    #[cfg(feature = "cache")]
    fn get_module_from_cache(
        &self,
        store: &Store,
        contents: &[u8],
        engine_type: &EngineType,
        compiler_type: &CompilerType,
    ) -> Result<Module> {
        // We try to get it from cache, in case caching is enabled
        // and the file length is greater than 4KB.
        // For files smaller than 4KB caching is not worth,
        // as it takes space and the speedup is minimal.
        let mut cache = self.get_cache(engine_type, compiler_type)?;
        // Try to get the hash from the provided `--cache-key`, otherwise
        // generate one from the provided file `.wasm` contents.
        let hash = self
            .cache_key
            .as_ref()
            .and_then(|key| Hash::from_str(&key).ok())
            .unwrap_or_else(|| Hash::generate(&contents));
        match unsafe { cache.load(&store, hash) } {
            Ok(module) => Ok(module),
            Err(e) => {
                match e {
                    DeserializeError::Io(_) => {
                        // Do not notify on IO errors
                    }
                    err => {
                        warning!("cached module is corrupted: {}", err);
                    }
                }
                let module = Module::new(&store, &contents)?;
                // Store the compiled Module in cache
                cache.store(hash, &module)?;
                Ok(module)
            }
        }
    }

    #[cfg(feature = "cache")]
    /// Get the Compiler Filesystem cache
    fn get_cache(
        &self,
        engine_type: &EngineType,
        compiler_type: &CompilerType,
    ) -> Result<FileSystemCache> {
        let mut cache_dir_root = get_cache_dir();
        cache_dir_root.push(compiler_type.to_string());
        let mut cache = FileSystemCache::new(cache_dir_root)?;

        // Important: Dylib files need to have a `.dll` extension on
        // Windows, otherwise they will not load, so we just add an
        // extension always to make it easier to recognize as well.
        #[allow(unreachable_patterns)]
        let extension = match *engine_type {
            #[cfg(feature = "dylib")]
            EngineType::Dylib => {
                wasmer_engine_dylib::DylibArtifact::get_default_extension(&Triple::host())
                    .to_string()
            }
            #[cfg(feature = "universal")]
            EngineType::Universal => {
                wasmer_engine_universal::UniversalArtifact::get_default_extension(&Triple::host())
                    .to_string()
            }
            // We use the compiler type as the default extension
            _ => compiler_type.to_string(),
        };
        cache.set_cache_extension(Some(extension));
        Ok(cache)
    }

    fn try_find_function(
        &self,
        instance: &Instance,
        name: &str,
        args: &[String],
    ) -> Result<Function> {
        Ok(instance
            .exports
            .get_function(&name)
            .map_err(|e| {
                if instance.module().info().functions.is_empty() {
                    anyhow!("The module has no exported functions to call.")
                } else {
                    let suggested_functions = suggest_function_exports(instance.module(), "");
                    let names = suggested_functions
                        .iter()
                        .take(3)
                        .map(|arg| format!("`{}`", arg))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let suggested_command = format!(
                        "wasmer {} -i {} {}",
                        self.path.display(),
                        suggested_functions.get(0).unwrap_or(&String::new()),
                        args.join(" ")
                    );
                    let suggestion = if suggested_functions.len() == 0 {
                        String::from("Can not find any export functions.")
                    } else {
                        format!(
                            "Similar functions found: {}.\nTry with: {}",
                            names, suggested_command
                        )
                    };
                    match e {
                        ExportError::Missing(_) => {
                            anyhow!("No export `{}` found in the module.\n{}", name, suggestion)
                        }
                        ExportError::IncompatibleType => anyhow!(
                            "Export `{}` found, but is not a function.\n{}",
                            name,
                            suggestion
                        ),
                    }
                }
            })?
            .clone())
    }

    fn invoke_function(
        &self,
        instance: &Instance,
        invoke: &str,
        args: &[String],
    ) -> Result<Box<[Val]>> {
        let func: Function = self.try_find_function(&instance, invoke, args)?;
        let func_ty = func.ty();
        let required_arguments = func_ty.params().len();
        let provided_arguments = args.len();
        if required_arguments != provided_arguments {
            bail!(
                "Function expected {} arguments, but received {}: \"{}\"",
                required_arguments,
                provided_arguments,
                self.args.join(" ")
            );
        }
        let invoke_args = args
            .iter()
            .zip(func_ty.params().iter())
            .map(|(arg, param_type)| match param_type {
                ValType::I32 => {
                    Ok(Val::I32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i32", arg)
                    })?))
                }
                ValType::I64 => {
                    Ok(Val::I64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i64", arg)
                    })?))
                }
                ValType::F32 => {
                    Ok(Val::F32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f32", arg)
                    })?))
                }
                ValType::F64 => {
                    Ok(Val::F64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f64", arg)
                    })?))
                }
                _ => Err(anyhow!(
                    "Don't know how to convert {} into {:?}",
                    arg,
                    param_type
                )),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(func.call(&invoke_args)?)
    }
}
