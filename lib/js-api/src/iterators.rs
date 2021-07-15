use wasmer_types::{
    ExportType, ExternType, FunctionType, GlobalType, ImportType, MemoryType, TableType,
};

// Code inspired from
// https://www.reddit.com/r/rust/comments/9vspv4/extending_iterators_ergonomically/

/// This iterator allows us to iterate over the exports
/// and offer nice API ergonomics over it.
pub struct ExportsIterator<I: Iterator<Item = ExportType> + Sized> {
    pub(crate) iter: I,
    pub(crate) size: usize,
}

impl<I: Iterator<Item = ExportType> + Sized> ExactSizeIterator for ExportsIterator<I> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.size
    }
}

impl<I: Iterator<Item = ExportType> + Sized> ExportsIterator<I> {
    /// Get only the functions
    pub fn functions(self) -> impl Iterator<Item = ExportType<FunctionType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Function(ty) => Some(ExportType::new(extern_.name(), ty.clone())),
            _ => None,
        })
    }
    /// Get only the memories
    pub fn memories(self) -> impl Iterator<Item = ExportType<MemoryType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Memory(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the tables
    pub fn tables(self) -> impl Iterator<Item = ExportType<TableType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Table(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the globals
    pub fn globals(self) -> impl Iterator<Item = ExportType<GlobalType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Global(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
}

impl<I: Iterator<Item = ExportType> + Sized> Iterator for ExportsIterator<I> {
    type Item = ExportType;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// This iterator allows us to iterate over the imports
/// and offer nice API ergonomics over it.
pub struct ImportsIterator<I: Iterator<Item = ImportType> + Sized> {
    pub(crate) iter: I,
    pub(crate) size: usize,
}

impl<I: Iterator<Item = ImportType> + Sized> ExactSizeIterator for ImportsIterator<I> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.size
    }
}

impl<I: Iterator<Item = ImportType> + Sized> ImportsIterator<I> {
    /// Get only the functions
    pub fn functions(self) -> impl Iterator<Item = ImportType<FunctionType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Function(ty) => Some(ImportType::new(
                extern_.module(),
                extern_.name(),
                ty.clone(),
            )),
            _ => None,
        })
    }
    /// Get only the memories
    pub fn memories(self) -> impl Iterator<Item = ImportType<MemoryType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Memory(ty) => Some(ImportType::new(extern_.module(), extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the tables
    pub fn tables(self) -> impl Iterator<Item = ImportType<TableType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Table(ty) => Some(ImportType::new(extern_.module(), extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the globals
    pub fn globals(self) -> impl Iterator<Item = ImportType<GlobalType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Global(ty) => Some(ImportType::new(extern_.module(), extern_.name(), *ty)),
            _ => None,
        })
    }
}

impl<I: Iterator<Item = ImportType> + Sized> Iterator for ImportsIterator<I> {
    type Item = ImportType;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
