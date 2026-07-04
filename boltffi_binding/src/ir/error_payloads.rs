use std::collections::BTreeSet;

use super::{
    CallbackDecl, CallbackProtocolIntrospect, ClassDecl, ConstantValueDecl, Decl, DeclarationRef,
    EnumDecl, EnumId, ErrorChannel, ExportedCallable, ExportedMethodDecl, FunctionDecl,
    ImportedCallable, IncomingParam, InitializerDecl, NativeSymbol, OutgoingParam, RecordDecl,
    RecordId, Surface, TypeRef,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ErrorPayloadTypes {
    records: BTreeSet<RecordId>,
    enumerations: BTreeSet<EnumId>,
}

impl ErrorPayloadTypes {
    pub(crate) fn from_decls<S: Surface>(decls: &[Decl<S>]) -> Self {
        decls
            .iter()
            .map(DeclarationRef::from)
            .fold(Self::default(), |mut payloads, declaration| {
                payloads.insert_declaration(declaration);
                payloads
            })
    }

    pub(crate) fn mark_decls<S: Surface>(&self, decls: &mut [Decl<S>]) {
        decls.iter_mut().for_each(|decl| match decl {
            Decl::Record(record) if self.records.contains(&record.id()) => {
                record.mark_error_payload();
            }
            Decl::Enum(enumeration) if self.enumerations.contains(&enumeration.id()) => {
                enumeration.mark_error_payload();
            }
            _ => {}
        });
    }

    fn insert_declaration<S: Surface>(&mut self, declaration: DeclarationRef<'_, S>) {
        match declaration {
            DeclarationRef::Function(function) => self.insert_function(function),
            DeclarationRef::Record(record) => self.insert_record(record),
            DeclarationRef::Enum(enumeration) => self.insert_enum(enumeration),
            DeclarationRef::Class(class) => self.insert_class(class),
            DeclarationRef::Constant(constant) => {
                if let ConstantValueDecl::Accessor { callable, .. } = constant.value() {
                    self.insert_exported_callable(callable);
                }
            }
            DeclarationRef::Callback(callback) => self.insert_callback(callback),
            DeclarationRef::Stream(_) | DeclarationRef::CustomType(_) => {}
        }
    }

    fn insert_function<S: Surface>(&mut self, function: &FunctionDecl<S>) {
        self.insert_exported_callable(function.callable());
    }

    fn insert_record<S: Surface>(&mut self, record: &RecordDecl<S>) {
        self.insert_associated(record.initializers(), record.methods());
    }

    fn insert_enum<S: Surface>(&mut self, enumeration: &EnumDecl<S>) {
        self.insert_associated(enumeration.initializers(), enumeration.methods());
    }

    fn insert_class<S: Surface>(&mut self, class: &ClassDecl<S>) {
        self.insert_associated(class.initializers(), class.methods());
    }

    fn insert_callback<S: Surface>(&mut self, callback: &CallbackDecl<S>) {
        callback
            .protocol()
            .method_callables()
            .for_each(|callable| self.insert_imported_callable(callable));
        if let Some(protocol) = callback.local_protocol() {
            protocol
                .methods()
                .iter()
                .for_each(|method| self.insert_exported_callable(method.callable()));
        }
    }

    fn insert_associated<S: Surface>(
        &mut self,
        initializers: &[InitializerDecl<S>],
        methods: &[ExportedMethodDecl<S, NativeSymbol>],
    ) {
        initializers
            .iter()
            .for_each(|initializer| self.insert_exported_callable(initializer.callable()));
        methods
            .iter()
            .for_each(|method| self.insert_exported_callable(method.callable()));
    }

    fn insert_exported_callable<S: Surface>(&mut self, callable: &ExportedCallable<S>) {
        if let ErrorChannel::Encoded { ty, .. } = callable.error().channel() {
            self.insert_type(ty);
        }
        callable.params().iter().for_each(|parameter| {
            if let IncomingParam::Closure(closure) = parameter.payload() {
                self.insert_imported_callable(closure.invoke());
            }
        });
    }

    fn insert_imported_callable<S: Surface>(&mut self, callable: &ImportedCallable<S>) {
        if let ErrorChannel::Encoded { ty, .. } = callable.error().channel() {
            self.insert_type(ty);
        }
        callable.params().iter().for_each(|parameter| {
            if let OutgoingParam::Closure(closure) = parameter.payload() {
                self.insert_exported_callable(closure.invoke());
            }
        });
    }

    fn insert_type(&mut self, ty: &TypeRef) {
        match ty {
            TypeRef::Record(record) => {
                self.records.insert(*record);
            }
            TypeRef::Enum(enumeration) => {
                self.enumerations.insert(*enumeration);
            }
            _ => {}
        }
    }
}
