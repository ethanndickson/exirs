use std::{
    mem::{self, MaybeUninit},
    os::raw::{c_char, c_uint, c_void},
};

use bytes::Bytes;

use crate::{
    data::{from_qname, from_stringtype, Attribute, Event, Name, Value},
    error::EXIPError,
    schema::Schema,
};

#[derive(Default)]
struct Handler<'a> {
    state: HandlerState<'a>,
}

#[derive(Default)]
enum HandlerState<'a> {
    #[default]
    Empty,
    PartialAttribute(Name<'a>),
    PartialList(Vec<Value<'a>>, u32),
    Event(Event<'a>),
}

impl<'a> HandlerState<'a> {
    fn take_event(&mut self) -> Event<'a> {
        let inner = mem::replace(self, HandlerState::Empty);
        match inner {
            HandlerState::Event(e) => e,
            _ => unreachable!("checked prior"),
        }
    }
}

impl<'a> Handler<'a> {
    fn start_document(&mut self) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::StartDocument);
        Ok(())
    }

    fn end_document(&mut self) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::EndDocument);
        Ok(())
    }

    fn start_element(
        &mut self,
        name: crate::data::Name<'a>,
    ) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::StartElement(name));
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::EndElement);
        Ok(())
    }

    fn attribute(&mut self, name: crate::data::Name<'a>) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::PartialAttribute(name);
        Ok(())
    }

    fn string(&mut self, value: &'a str) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::String(value)));
        Ok(())
    }

    fn decimal(&mut self, value: ffi::EXIFloat) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::Float(value.into())));
        Ok(())
    }

    fn boolean(&mut self, value: bool) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::Boolean(value)));
        Ok(())
    }

    fn datetime(&mut self, dt: &'a chrono::NaiveDateTime) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::Timestamp(dt)));
        Ok(())
    }

    fn binary(&mut self, bytes: &'a [u8]) -> Result<(), crate::error::EXIPError> {
        // EXIP immediately frees read bytes, so we need to copy
        self.state =
            HandlerState::Event(Event::Value(Value::Binary(Bytes::copy_from_slice(bytes))));
        Ok(())
    }

    fn qname(&mut self, name: crate::data::Name<'a>) -> Result<(), crate::error::EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::QName(name)));
        Ok(())
    }

    fn int(&mut self, int: i64) -> Result<(), EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::Integer(int)));
        Ok(())
    }

    fn float(&mut self, value: ffi::EXIFloat) -> Result<(), EXIPError> {
        self.state = HandlerState::Event(Event::Value(Value::Float(value.into())));
        Ok(())
    }

    fn list(&mut self, len: u32) -> Result<(), EXIPError> {
        self.state = HandlerState::PartialList(vec![], len);
        Ok(())
    }

    fn processing_instruction(&mut self) -> Result<(), EXIPError> {
        todo!();
    }

    fn namespace_declaration(&mut self, _: &str, _: &str, _: bool) -> Result<(), EXIPError> {
        todo!();
    }

    fn warning(&mut self, _: EXIPError, _: &str) -> Result<(), EXIPError> {
        todo!();
    }

    fn error(&mut self, _: EXIPError, _: &str) -> Result<(), EXIPError> {
        todo!();
    }

    fn self_contained(&mut self) -> Result<(), EXIPError> {
        todo!();
    }
}
pub struct Reader<'a> {
    uses_schema: bool,
    parser: Box<ffi::Parser>,
    _buf: Box<[u8]>,
    handler: Box<Handler<'a>>,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: impl AsRef<[u8]>, schema: Option<Schema>) -> Result<Self, EXIPError> {
        let uses_schema = schema.is_some();
        let mut parser: MaybeUninit<ffi::Parser> = MaybeUninit::uninit();
        let mut heap_buf: Box<[u8]> = Box::from(bytes.as_ref());
        let buf_rep = ffi::BinaryBuffer {
            buf: heap_buf.as_mut_ptr() as *mut _,
            bufLen: heap_buf.len(),
            bufContent: heap_buf.len(),
            ioStrm: ffi::ioStream {
                readWriteToStream: None,
                stream: std::ptr::null_mut(),
            },
        };
        let handler = Box::<Handler>::default();
        let ec = unsafe {
            (ffi::parse.initParser).unwrap()(
                parser.as_mut_ptr(),
                buf_rep,
                &*handler as *const _ as *mut _,
            )
        };
        assert_eq!(ec, 0);
        let mut parser = unsafe { parser.assume_init() };
        parser.handler = new_handler();
        let ec = unsafe { (ffi::parse.parseHeader).unwrap()(&mut parser as *mut _, 0) };
        assert_eq!(ec, 0);
        let ec = unsafe {
            (ffi::parse.setSchema).unwrap()(
                &mut parser as *mut _,
                schema.map_or(std::ptr::null_mut(), |mut s| s.inner.as_mut()),
            )
        };
        if ec != 0 {
            return Err(ec.into());
        }
        Ok(Self {
            parser: Box::new(parser),
            _buf: heap_buf,
            handler,
            uses_schema,
        })
    }
}

impl<'a> Drop for Reader<'a> {
    fn drop(&mut self) {
        unsafe { (ffi::parse.destroyParser).unwrap()(self.parser.as_mut() as *mut _) }
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Result<Event<'a>, EXIPError>;

    fn next(&mut self) -> Option<Self::Item> {
        match mem::replace(&mut self.handler.state, HandlerState::Empty) {
            HandlerState::Event(Event::StartDocument) => Some(Ok(Event::StartDocument)),
            HandlerState::Event(Event::EndDocument) => None,
            HandlerState::Empty => {
                let ec = unsafe { (ffi::parse.parseNext).unwrap()(self.parser.as_mut()) };
                match ec {
                    ffi::errorCode_EXIP_OK => match &self.handler.state {
                        HandlerState::PartialAttribute(_) | HandlerState::PartialList(_, _) => {
                            self.next()
                        }
                        _ => Some(Ok(self.handler.state.take_event())),
                    },
                    ffi::errorCode_EXIP_PARSING_COMPLETE => Some(Ok(Event::EndDocument)),
                    e => Some(Err(e.into())),
                }
            }
            HandlerState::PartialAttribute(name) => match self.next()? {
                Ok(Event::Value(value)) => {
                    Some(Ok(Event::Attribute(Attribute { key: name, value })))
                }
                Ok(_) => Some(Err(EXIPError::Unexpected)),
                Err(e) => Some(Err(e)),
            },
            HandlerState::PartialList(mut vec, length) => match self.next()? {
                Ok(Event::Value(value)) => {
                    vec.push(value);
                    if vec.len() == length as usize {
                        return Some(Ok(Event::Value(Value::List(vec))));
                    } else {
                        self.handler.state = HandlerState::PartialList(vec, length);
                        self.next()
                    }
                }
                Ok(_) => Some(Err(EXIPError::Unexpected)),
                Err(e) => Some(Err(e)),
            },
            _ => Some(Err(EXIPError::Unexpected)),
        }
    }
}

unsafe extern "C" fn invoke_start_document(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.start_document() {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_end_document(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.end_document() {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_start_element(
    qname: ffi::QName,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.start_element(from_qname(qname)) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_end_element(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.end_element() {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_attribute(qname: ffi::QName, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.attribute(from_qname(qname)) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_int(integer: ffi::Integer, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.int(integer) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_boolean(bool: ffi::boolean, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.boolean(bool != 0) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_string(str: ffi::String, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.string(from_stringtype(&str as *const _).unwrap_or_default()) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_float(float: ffi::Float, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.float(float) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_binary(
    binary: *const c_char,
    nbytes: usize,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    let slice = std::slice::from_raw_parts(binary as *const u8, nbytes);
    match handler.binary(slice) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_datetime(
    dt_val: ffi::EXIPDateTime,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match &chrono::NaiveDateTime::try_from(&dt_val) {
        // todo: this could return a date in another format if it's invalid
        Ok(dt) => match handler.datetime(dt) {
            Ok(_) => ffi::errorCode_EXIP_OK,
            Err(e) => e as u32,
        },
        Err(_) => return ffi::errorCode_EXIP_INVALID_EXI_INPUT,
    }
}

unsafe extern "C" fn invoke_decimal(val: ffi::Decimal, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.decimal(val) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_list(
    _: ffi::EXITypeClass,
    item_count: c_uint,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.list(item_count) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_qname(qname: ffi::QName, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.qname(from_qname(qname)) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_processing_instruction(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_nsdec(
    ns: ffi::String,
    prefix: ffi::String,
    is_local: c_uint,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_warning(
    code: ffi::errorCode,
    msg: *const c_char,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_error(
    code: ffi::errorCode,
    msg: *const c_char,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_fatal_error(
    code: ffi::errorCode,
    msg: *const c_char,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_self_contained(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

fn new_handler() -> ffi::ContentHandler {
    ffi::ContentHandler {
        startDocument: Some(invoke_start_document),
        endDocument: Some(invoke_end_document),
        startElement: Some(invoke_start_element),
        endElement: Some(invoke_end_element),
        attribute: Some(invoke_attribute),
        intData: Some(invoke_int),
        booleanData: Some(invoke_boolean),
        stringData: Some(invoke_string),
        floatData: Some(invoke_float),
        binaryData: Some(invoke_binary),
        dateTimeData: Some(invoke_datetime),
        decimalData: Some(invoke_decimal),
        listData: Some(invoke_list),
        qnameData: Some(invoke_qname),
        processingInstruction: Some(invoke_processing_instruction),
        namespaceDeclaration: Some(invoke_nsdec),
        warning: Some(invoke_warning),
        error: Some(invoke_error),
        fatalError: Some(invoke_fatal_error),
        selfContained: Some(invoke_self_contained),
    }
}

#[test]
fn simple_read() {
    use crate::data::Name;

    let input = [
        36, 69, 88, 73, 160, 2, 172, 2, 12, 178, 18, 52, 58, 58, 56, 29, 23, 151, 187, 187, 187,
        151, 54, 58, 58, 151, 57, 178, 151, 162, 164, 169, 166, 32, 161, 23, 185, 177, 180, 50,
        182, 176, 150, 186, 50, 185, 186, 8, 166, 186, 182, 58, 52, 184, 54, 50, 172, 41, 162, 57,
        170, 50, 185, 186, 105, 10, 141, 13, 46, 100, 13, 46, 100, 12, 45, 196, 12, 175, 12, 45,
        174, 13, 140, 164, 13, 236, 196, 14, 108, 174, 77, 44, 45, 141, 47, 77, 45, 204, 228, 8,
        171, 9, 36, 14, 110, 142, 76, 172, 45, 174, 100, 14, 174, 109, 45, 204, 228, 8, 171, 9, 42,
        4, 13, 141, 238, 228, 13, 140, 174, 204, 173, 132, 8, 42, 9, 32,
    ];
    let mut reader = Reader::new(input, None).unwrap();
    assert_eq!(reader.next(), Some(Ok(Event::StartDocument)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::String(
            "This is an example of serializing EXI streams using EXIP low level API"
        ))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndDocument)));
    assert_eq!(reader.next(), None);
}

#[test]
fn full_read() {
    let input = [
        0x24, 0x45, 0x58, 0x49, 0xA0, 0x02, 0xAC, 0x02, 0x0C, 0xB2, 0x96, 0xE0, 0x53, 0x02, 0xE3,
        0x24, 0x85, 0x46, 0x86, 0x97, 0x32, 0x06, 0x97, 0x32, 0x06, 0x16, 0xE2, 0x06, 0x57, 0x86,
        0x16, 0xD7, 0x06, 0xC6, 0x52, 0x06, 0xF6, 0x62, 0x07, 0x36, 0x57, 0x26, 0x96, 0x16, 0xC6,
        0x97, 0xA6, 0x96, 0xE6, 0x72, 0x04, 0x55, 0x84, 0x92, 0x07, 0x37, 0x47, 0x26, 0x56, 0x16,
        0xD7, 0x32, 0x07, 0x57, 0x36, 0x96, 0xE6, 0x72, 0x04, 0x55, 0x84, 0x95, 0x02, 0x06, 0xC6,
        0xF7, 0x72, 0x06, 0xC6, 0x57, 0x66, 0x56, 0xC2, 0x04, 0x15, 0x04, 0x92, 0x12, 0xA3, 0x43,
        0x4B, 0x99, 0x03, 0x4B, 0x99, 0x03, 0x09, 0x03, 0xA3, 0x2B, 0x9B, 0xA1, 0x03, 0x7B, 0x31,
        0x03, 0x83, 0x93, 0x7B, 0x1B, 0x2B, 0x9B, 0x9B, 0x4B, 0x73, 0x39, 0x02, 0xC2, 0x6A, 0x61,
        0x03, 0x9B, 0x1B, 0x43, 0x2B, 0x6B, 0x2B, 0x99, 0x03, 0xBB, 0x4B, 0xA3, 0x41, 0x03, 0x6B,
        0xAB, 0x63, 0xA3, 0x4B, 0x83, 0x63, 0x29, 0x02, 0xC2, 0x9A, 0x21, 0x03, 0x33, 0x4B, 0x63,
        0x2B, 0x98, 0x9D, 0x59, 0x95, 0xC9, 0xA5, 0x99, 0xE4, 0x81, 0xD1, 0xA1, 0x85, 0xD0, 0x81,
        0xD1, 0xA1, 0x94, 0x81, 0xA5, 0xB5, 0xC1, 0xB1, 0x95, 0xB5, 0x95, 0xB9, 0xD1, 0x85, 0xD1,
        0xA5, 0xBD, 0xB8, 0x81, 0xDD, 0xBD, 0xC9, 0xAD, 0xCC, 0x84, 0xAD, 0x4D, 0xA5, 0xB5, 0xC1,
        0xB1, 0x94, 0x81, 0xD1, 0x95, 0xCD, 0xD0, 0x81, 0x95, 0xB1, 0x95, 0xB5, 0x95, 0xB9, 0xD0,
        0x81, 0xDD, 0xA5, 0xD1, 0xA0, 0x81, 0xCD, 0xA5, 0xB9, 0x9D, 0xB1, 0x94, 0x81, 0x85, 0xD1,
        0xD1, 0xC9, 0xA5, 0x89, 0xD5, 0xD1, 0x94, 0x74, 0x83, 0xA8, 0xB0, 0x63, 0xFD, 0xB0, 0xEF,
        0x90, 0xA0, 0x39, 0x01, 0x40, 0x4D, 0xA5, 0xF4, 0xA4, 0x1E, 0x4C, 0x33, 0x9D, 0xC1, 0xEC,
    ];
    let schema = Schema::new(
        &[
            "./examples/exipe-test-xsd.exi",
            "./examples/exipe-test-types-xsd.exi",
            "./examples/exipe-test-nested-xsd.exi",
        ],
        None,
    )
    .unwrap();
    let mut reader = Reader::new(input, Some(schema)).unwrap();
    assert_eq!(reader.next(), Some(Ok(Event::StartDocument)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "EXIPEncoder",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Integer(55)))));
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::String("0.2")))));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::String(
            "This is an example of serializing EXI streams using EXIP low level API"
        ))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "description",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::String(
            "This is a test of processing XML schemes with multiple XSD files"
        ))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "testSetup",
            namespace: "http://www.ltu.se/EISLAB/nested-xsd",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::String(
            "Verify that the implementation works!"
        ))))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::String(
            "Simple test element with single attribute"
        ))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "type-test",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Integer(1001)))));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "bool",
            namespace: "http://www.ltu.se/EISLAB/nested-xsd",
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Boolean(true)))));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "extendedTypeTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "byteTest",
            namespace: "",
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Integer(11)))));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "dateTimeTest",
            namespace: "",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::Timestamp(
            &chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2012, 7, 31).unwrap(),
                chrono::NaiveTime::from_hms_micro_opt(13, 33, 55, 839).unwrap(),
            )
        ))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "binaryTest",
            namespace: "",
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::Value(Value::Binary(Bytes::from_static(&[
            0x02, 0x6d, 0x2f, 0xa5, 0x20, 0xf2, 0x61, 0x9c, 0xee, 0x0f,
        ])))))
    );
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "enumTest",
            namespace: "",
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::String("hej")))));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndDocument)));
}
