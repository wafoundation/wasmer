use crate::store::StoreOptions;
use crate::warning;
use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use wasmer::*;

#[derive(Debug, StructOpt)]
/// The options for the `wasmer compile` subcommand
pub struct Compile {
    /// Input file
    #[structopt(name = "FILE", parse(from_os_str))]
    path: PathBuf,

    /// Output file
    #[structopt(name = "OUTPUT", short = "o", parse(from_os_str))]
    output: PathBuf,

    #[structopt(flatten)]
    compiler: StoreOptions,
}

impl Compile {
    /// Runs logic for the `compile` subcommand
    pub fn execute(&self) -> Result<()> {
        self.inner_execute()
            .context(format!("failed to compile `{}`", self.path.display()))
    }
    fn inner_execute(&self) -> Result<()> {
        let (store, engine_name, compiler_name) = self.compiler.get_store()?;
        let output_filename = self
            .output
            .file_stem()
            .map(|osstr| osstr.to_string_lossy().to_string())
            .unwrap_or_default();
        let target = self.compiler.get_target()?;
        let recommended_extension = match engine_name.as_ref() {
            "native" => {
                // TODO: Match it depending on the `BinaryFormat` instead of the
                // `OperatingSystem`.
                match target.triple().operating_system {
                    OperatingSystem::Darwin
                    | OperatingSystem::Ios
                    | OperatingSystem::MacOSX { .. } => "dylib",
                    _ => "so",
                }
            }
            "jit" => "wjit",
            _ => "?",
        };
        match self.output.extension() {
            Some(ext) => {
                if ext != recommended_extension {
                    warning!("the output file has a wrong extension. We recommend using `{}.{}` for the chosen target", &output_filename, &recommended_extension)
                }
            },
            None => {
                warning!("the output file has no extension. We recommend using `{}.{}` for the chosen target", &output_filename, &recommended_extension)
            }
        }
        println!("Engine: {}", engine_name);
        println!("Compiler: {}", compiler_name);
        println!("Target: {}", target.triple());
        let module = Module::from_file(&store, &self.path)?;
        let _ = module.serialize_to_file(&self.output)?;
        eprintln!(
            "✔ File compiled successfully to `{}`.",
            self.output.display(),
        );
        Ok(())
    }
}