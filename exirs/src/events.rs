use crate::data::{Name, SchemaAttribute, SchemalessAttribute};

#[non_exhaustive]
// No processingInstruction support
pub enum SchemalessEvent<'a> {
    StartDocument,
    EndDocument,
    StartElement(Name<'a>),
    EndElement,
    Attribute(SchemalessAttribute<'a>),
    NamespaceDeclaration {
        namespace: &'a str,
        prefix: &'a str,
        is_local: bool,
    },
    ExiHeader,
    SelfContained,
}

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
    SelfContained,
}
