use std::{fmt::Display, time::SystemTime};

use base64::Engine;
use ffi::StringType;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
// No processingInstruction or selfContained support

pub enum Event<'a> {
    StartDocument,
    EndDocument,
    StartElement(Name<'a>),
    EndElement,
    TypeAttribute(Name<'a>),
    Attribute(Attribute<'a>),
    NamespaceDeclaration(NamespaceDeclaration<'a>),
    Value(Value<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamespaceDeclaration<'a> {
    pub namespace: &'a str,
    pub prefix: &'a str,
    pub is_local_element: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attribute<'a> {
    pub key: Name<'a>,
    pub value: Value<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Name<'a> {
    pub local_name: &'a str,
    pub namespace: &'a str,
    pub prefix: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Integer(i64),
    Boolean(bool),
    String(&'a str),
    Float(f64),
    Binary(&'a [u8]),
    Timestamp(&'a SystemTime),
    List(&'a [Value<'a>]),
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(int) => write!(f, "{}", int),
            Value::Boolean(bool) => write!(f, "{}", bool),
            Value::String(str) => write!(f, "{}", str),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Binary(bin) => write!(
                f,
                "{}",
                base64::engine::general_purpose::STANDARD.encode(bin)
            ),
            Value::List(list) => write!(
                f,
                "{}",
                list.iter().map(|i| i.to_string()).collect::<String>()
            ),
            Value::Timestamp(_) => todo!(),
        }
    }
}

pub(crate) fn to_stringtype(str: &str) -> ffi::StringType {
    StringType {
        str_: str.as_ptr() as *mut _,
        length: str.len(),
    }
}

pub(crate) fn from_stringtype<'a>(str: *const ffi::StringType) -> Option<&'a str> {
    if str.is_null() {
        None
    } else {
        let slice = unsafe { std::slice::from_raw_parts((*str).str_ as *const u8, (*str).length) };
        std::str::from_utf8(slice).ok()
    }
}

pub(crate) fn from_qname<'a>(qname: ffi::QName) -> Name<'a> {
    Name {
        local_name: from_stringtype(qname.localName).unwrap_or_default(),
        namespace: from_stringtype(qname.uri).unwrap_or_default(),
        prefix: from_stringtype(qname.prefix),
    }
}

pub(crate) fn to_qname(name: Name) -> ffi::QName {
    ffi::QName {
        uri: &to_stringtype(name.namespace),
        localName: &to_stringtype(name.local_name),
        prefix: match name.prefix {
            Some(n) => &to_stringtype(n),
            None => std::ptr::null(),
        },
    }
}
