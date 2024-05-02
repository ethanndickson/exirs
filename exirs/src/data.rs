use std::time::SystemTime;

use ffi::StringType;

#[derive(Debug)]
pub struct SchemalessAttribute<'a> {
    pub key: Name<'a>,
    pub value: &'a str,
}

#[derive(Debug)]
pub struct NamespaceDeclaration<'a> {
    pub namespace: &'a str,
    pub prefix: &'a str,
    pub is_local_element: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SchemaAttribute<'a> {
    pub key: Name<'a>,
    pub value: SchemaValue<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Name<'a> {
    pub local_name: &'a str,
    pub namespace: &'a str,
    pub prefix: Option<&'a str>,
}

pub struct Float {
    pub mantissa: i64,
    pub exponent: i16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SchemaValue<'a> {
    Integer(i64),
    Boolean(bool),
    String(&'a str),
    Float(f64),
    Binary(&'a [u8]),
    Timestamp(&'a SystemTime),
    List(&'a [SchemaValue<'a>]),
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
