use crate::data::{Name, NamespaceDeclaration, SchemaAttribute, SchemaValue, SchemalessAttribute};

#[derive(Debug)]
#[non_exhaustive]
// No processingInstruction or selfContained support
pub enum SchemalessEvent<'a> {
    StartDocument,
    EndDocument,
    StartElement(Name<'a>),
    EndElement,
    Attribute(SchemalessAttribute<'a>),
    NamespaceDeclaration(NamespaceDeclaration<'a>),
    ExiHeader,
    Characters(&'a str),
}

#[derive(Debug)]
#[non_exhaustive]
// No processingInstruction or selfContained support
pub enum SchemaEvent<'a> {
    StartDocument,
    EndDocument,
    StartElement(Name<'a>),
    EndElement,
    Attribute(SchemaAttribute<'a>),
    NamespaceDeclaration {
        namespace: &'a str,
        prefix: &'a str,
        is_local: bool,
    },
    ExiHeader,
    Value(SchemaValue<'a>),
}
