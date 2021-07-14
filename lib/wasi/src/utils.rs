use std::collections::BTreeSet;
use wasmer::Module;

#[allow(dead_code)]
/// Check if a provided module is compiled for some version of WASI.
/// Use [`get_wasi_version`] to find out which version of WASI the module is.
pub fn is_wasi_module(module: &Module) -> bool {
    get_wasi_version(module, false).is_some()
}

/// The version of WASI. This is determined by the imports namespace
/// string.
#[derive(Debug, Clone, Copy, Eq)]
pub enum WasiVersion {
    /// `wasi_unstable`.
    Snapshot0,

    /// `wasi_snapshot_preview1`.
    Snapshot1,

    /// Latest version.
    ///
    /// It's a “floating” version, i.e. it's an alias to the latest
    /// version (for the moment, `Snapshot1`). Using this version is a
    /// way to ensure that modules will run only if they come with the
    /// latest WASI version (in case of security issues for instance),
    /// by just updating the runtime.
    ///
    /// Note that this version is never returned by an API. It is
    /// provided only by the user.
    Latest,
}

impl WasiVersion {
    /// Get the version as its namespace str as it appears in Wasm modules.
    pub const fn get_namespace_str(&self) -> &'static str {
        match *self {
            WasiVersion::Snapshot0 => SNAPSHOT0_NAMESPACE,
            WasiVersion::Snapshot1 => SNAPSHOT1_NAMESPACE,
            WasiVersion::Latest => SNAPSHOT1_NAMESPACE,
        }
    }
}

impl PartialEq<WasiVersion> for WasiVersion {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Snapshot1, Self::Latest)
            | (Self::Latest, Self::Snapshot1)
            | (Self::Latest, Self::Latest)
            | (Self::Snapshot0, Self::Snapshot0)
            | (Self::Snapshot1, Self::Snapshot1) => true,
            _ => false,
        }
    }
}

impl PartialOrd for WasiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WasiVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            return std::cmp::Ordering::Equal;
        }
        match (*self, *other) {
            // if snapshot0 is not equal, it must be less
            (Self::Snapshot0, _) => std::cmp::Ordering::Less,
            (Self::Snapshot1, Self::Snapshot0) | (Self::Latest, Self::Snapshot0) => {
                std::cmp::Ordering::Greater
            }
            _ => unreachable!("Missing info about ordering of WasiVerison"),
        }
    }
}

/// Namespace for the `Snapshot0` version.
const SNAPSHOT0_NAMESPACE: &str = "wasi_unstable";

/// Namespace for the `Snapshot1` version.
const SNAPSHOT1_NAMESPACE: &str = "wasi_snapshot_preview1";

/// Detect the version of WASI being used based on the import
/// namespaces.
///
/// A strict detection expects that all imports live in a single WASI
/// namespace. A non-strict detection expects that at least one WASI
/// namespace exists to detect the version. Note that the strict
/// detection is faster than the non-strict one.
pub fn get_wasi_version(module: &Module, strict: bool) -> Option<WasiVersion> {
    let mut imports = module.imports().functions().map(|f| f.module().to_owned());

    if strict {
        let first_module = imports.next()?;
        if imports.all(|module| module == first_module) {
            match first_module.as_str() {
                SNAPSHOT0_NAMESPACE => Some(WasiVersion::Snapshot0),
                SNAPSHOT1_NAMESPACE => Some(WasiVersion::Snapshot1),
                _ => None,
            }
        } else {
            None
        }
    } else {
        // Check that at least a WASI namespace exists, and use the
        // first one in the list to detect the WASI version.
        imports.find_map(|module| match module.as_str() {
            SNAPSHOT0_NAMESPACE => Some(WasiVersion::Snapshot0),
            SNAPSHOT1_NAMESPACE => Some(WasiVersion::Snapshot1),
            _ => None,
        })
    }
}

/// Returns the wasi versions found in `module`
pub fn get_wasi_versions(module: &Module) -> BTreeSet<WasiVersion> {
    let mut out = BTreeSet::new();
    let imports = module.imports().functions().map(|f| f.module().to_owned());

    for ns in imports {
        match ns.as_str() {
            SNAPSHOT0_NAMESPACE => {
                out.insert(WasiVersion::Snapshot0);
            }
            SNAPSHOT1_NAMESPACE => {
                out.insert(WasiVersion::Snapshot1);
            }
            _ => {
                // noop
            }
        }
    }

    out
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn wasi_version_equality() {
        assert_eq!(WasiVersion::Snapshot0, WasiVersion::Snapshot0);
        assert_eq!(WasiVersion::Snapshot1, WasiVersion::Snapshot1);
        assert_eq!(WasiVersion::Snapshot1, WasiVersion::Latest);
        assert_eq!(WasiVersion::Latest, WasiVersion::Snapshot1);
        assert_eq!(WasiVersion::Latest, WasiVersion::Latest);
        assert!(WasiVersion::Snapshot0 != WasiVersion::Snapshot1);
        assert!(WasiVersion::Snapshot1 != WasiVersion::Snapshot0);
        assert!(WasiVersion::Snapshot0 != WasiVersion::Latest);
        assert!(WasiVersion::Latest != WasiVersion::Snapshot0);
    }

    #[test]
    fn wasi_version_ordering() {
        assert!(WasiVersion::Snapshot0 <= WasiVersion::Snapshot0);
        assert!(WasiVersion::Snapshot1 <= WasiVersion::Snapshot1);
        assert!(WasiVersion::Latest <= WasiVersion::Latest);
        assert!(WasiVersion::Snapshot0 >= WasiVersion::Snapshot0);
        assert!(WasiVersion::Snapshot1 >= WasiVersion::Snapshot1);
        assert!(WasiVersion::Latest >= WasiVersion::Latest);

        assert!(WasiVersion::Snapshot0 < WasiVersion::Snapshot1);
        assert!(WasiVersion::Snapshot1 > WasiVersion::Snapshot0);
        assert!(WasiVersion::Snapshot0 < WasiVersion::Latest);
        assert!(WasiVersion::Latest > WasiVersion::Snapshot0);

        assert!(!(WasiVersion::Snapshot1 < WasiVersion::Latest));
        assert!(!(WasiVersion::Latest > WasiVersion::Snapshot1));
    }
}
