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

#[derive(Debug)]
pub struct SchemaAttribute<'a> {
    pub key: Name<'a>,
    pub value: SchemaValue<'a>,
}

#[derive(Debug)]
pub struct Name<'a> {
    pub local_name: &'a str,
    pub namespace: &'a str,
    pub prefix: Option<&'a str>,
}

#[derive(Clone, Debug)]
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
