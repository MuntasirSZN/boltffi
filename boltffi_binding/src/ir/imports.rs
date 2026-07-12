use crate::{
    Bindings, Decl, ExportedCallable, ImportSymbol, ImportedCallable, IncomingParam, OutgoingParam,
    ReturnPlan, Wasm32,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// The imports required to instantiate one Wasm binding contract.
pub struct WasmImports<'bindings> {
    symbols: Vec<&'bindings ImportSymbol>,
}

impl<'bindings> WasmImports<'bindings> {
    /// Collects every callback and closure import in declaration order.
    pub fn from_bindings(bindings: &'bindings Bindings<Wasm32>) -> Self {
        bindings
            .decls()
            .iter()
            .fold(Self::default(), |mut imports, declaration| {
                declaration
                    .exported_callables()
                    .for_each(|callable| imports.insert_exported(callable));
                declaration
                    .imported_callables()
                    .for_each(|callable| imports.insert_imported(callable));
                if let Decl::Callback(callback) = declaration {
                    let protocol = callback.protocol();
                    imports.insert(protocol.free());
                    imports.insert(protocol.clone_import());
                    protocol
                        .methods()
                        .iter()
                        .for_each(|method| imports.insert(method.target()));
                }
                imports
            })
    }

    /// Returns the imports in first-use order.
    pub fn iter(&self) -> impl Iterator<Item = &'bindings ImportSymbol> + '_ {
        self.symbols.iter().copied()
    }

    fn insert_exported(&mut self, callable: &'bindings ExportedCallable<Wasm32>) {
        callable.params().iter().for_each(|parameter| {
            if let IncomingParam::Closure(closure) = parameter.payload() {
                let registration = closure.registration().shape();
                self.insert(registration.call());
                self.insert(registration.free());
                self.insert_imported(closure.invoke());
            }
        });
        if let ReturnPlan::ClosureViaOutPointer(closure) = callable.returns().plan() {
            self.insert_exported(closure.invoke());
        }
    }

    fn insert_imported(&mut self, callable: &'bindings ImportedCallable<Wasm32>) {
        callable.params().iter().for_each(|parameter| {
            if let OutgoingParam::Closure(closure) = parameter.payload() {
                self.insert_exported(closure.invoke());
            }
        });
        if let ReturnPlan::ClosureViaOutPointer(closure) = callable.returns().plan() {
            let registration = closure.registration().shape();
            self.insert(registration.call());
            self.insert(registration.free());
            self.insert_imported(closure.invoke());
        }
    }

    fn insert(&mut self, symbol: &'bindings ImportSymbol) {
        if !self.symbols.iter().any(|existing| {
            existing.module().as_str() == symbol.module().as_str()
                && existing.name().as_str() == symbol.name().as_str()
        }) {
            self.symbols.push(symbol);
        }
    }
}
