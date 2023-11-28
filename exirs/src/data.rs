use std::{borrow::Cow, time::SystemTime};

pub struct SchemalessAttribute<'a> {
    pub key: Name<'a>,
    pub value: Cow<'a, [u8]>,
}

pub struct SchemaAttribute<'a> {
    pub key: Name<'a>,
    pub value: Cow<'a, SchemaValue<'a>>,
}

pub struct Name<'a> {
    pub local_name: &'a str,
    pub namespace: &'a str,
    pub prefix: Option<&'a str>,
}

#[derive(Clone)]
pub enum SchemaValue<'a> {
    Integer(i64),
    Boolean(bool),
    String(&'a str),
    Float(f64),
    Binary(&'a [u8]),
    Timestamp(&'a SystemTime),
    List(&'a [SchemaValue<'a>]),
}
