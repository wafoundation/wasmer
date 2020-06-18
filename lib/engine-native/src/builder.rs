use crate::NativeEngine;
use wasmer_compiler::{CompilerConfig, Features, Target};

/// The Native builder
pub struct Native<'a> {
    compiler_config: Option<&'a dyn CompilerConfig>,
    target: Option<Target>,
    features: Option<Features>,
}

impl<'a> Native<'a> {
    /// Create a new Native
    pub fn new(compiler_config: &'a mut dyn CompilerConfig) -> Self {
        compiler_config.enable_pic();
        Self {
            compiler_config: Some(compiler_config),
            target: None,
            features: None,
        }
    }

    /// Create a new headless Native
    pub fn headless() -> Self {
        Self {
            compiler_config: None,
            target: None,
            features: None,
        }
    }

    /// Set the target
    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the features
    pub fn features(mut self, features: Features) -> Self {
        self.features = Some(features);
        self
    }

    /// Build the `NativeEngine` for this configuration
    pub fn engine(self) -> NativeEngine {
        let target = self.target.unwrap_or_default();
        if let Some(compiler_config) = self.compiler_config {
            let features = self
                .features
                .unwrap_or_else(|| compiler_config.default_features_for_target(&target));
            let compiler = compiler_config.compiler();
            NativeEngine::new(compiler, target, features)
        } else {
            NativeEngine::headless()
        }
    }
}