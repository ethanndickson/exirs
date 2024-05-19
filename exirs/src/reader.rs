use std::{
    io::Read,
    mem::{self, MaybeUninit},
    os::raw::{c_char, c_uint, c_void},
};

use bytes::Bytes;

use crate::{
    config::{Options, Schema},
    data::{from_qname, from_stringtype, Attribute, Event, Name, NamespaceDeclaration, Value},
    error::{EXIPError, ReaderError},
};

const INPUT_BUFFER_SIZE: usize = 8 * 1024;

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

    fn namespace_declaration(
        &mut self,
        namespace: &'a str,
        prefix: &'a str,
        is_local_element: bool,
    ) -> Result<(), EXIPError> {
        self.state = HandlerState::Event(Event::NamespaceDeclaration(NamespaceDeclaration {
            namespace,
            prefix,
            is_local_element,
        }));
        Ok(())
    }
}
pub struct Reader<'a, R: Read> {
    parser: Box<ffi::Parser>,
    _buf: Box<[u8; INPUT_BUFFER_SIZE]>,
    handler: Box<Handler<'a>>,
    source: R,
}

impl<'a, R: Read> Reader<'a, R> {
    /// if `bytes` is buffered, it'll get double buffered.
    ///
    /// If a `crate::config::Options` is supplied, it will be used when not found in the EXI header
    pub fn new(
        mut source: R,
        schema: Option<Schema>,
        options: Option<Options>,
    ) -> Result<Self, ReaderError> {
        let has_options = options.is_some() as u32;
        let mut parser: MaybeUninit<ffi::Parser> = MaybeUninit::uninit();
        let mut heap_buf = Box::new([0u8; INPUT_BUFFER_SIZE]);
        source
            .read(&mut heap_buf.as_mut_slice())
            .map_err(|e| ReaderError::IO(e))?;
        let buf_rep = ffi::BinaryBuffer {
            buf: heap_buf.as_ptr() as *mut _,
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
        if let Some(options) = options {
            parser.strm.header.opts = options.ffi()
        }
        let ec = unsafe { (ffi::parse.parseHeader).unwrap()(&mut parser as *mut _, has_options) };
        assert_eq!(ec, 0);
        let ec = unsafe {
            (ffi::parse.setSchema).unwrap()(
                &mut parser as *mut _,
                schema.map_or(std::ptr::null_mut(), |mut s| s.inner.as_mut()),
            )
        };
        if ec != 0 {
            return Err(ReaderError::EXIP(ec.into()));
        }
        Ok(Self {
            source,
            parser: Box::new(parser),
            _buf: heap_buf,
            handler,
        })
    }

    fn pull(&mut self) {
        todo!()
    }
}

impl<'a, R: Read> Drop for Reader<'a, R> {
    fn drop(&mut self) {
        unsafe { (ffi::parse.destroyParser).unwrap()(self.parser.as_mut() as *mut _) }
    }
}

impl<'a, R: Read> Iterator for Reader<'a, R> {
    type Item = Result<Event<'a>, ReaderError>;

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
                    ffi::errorCode_EXIP_BUFFER_END_REACHED => {
                        self.pull();
                        self.next()
                    }
                    ffi::errorCode_EXIP_PARSING_COMPLETE => Some(Ok(Event::EndDocument)),
                    e => Some(Err(ReaderError::EXIP(e.into()))),
                }
            }
            HandlerState::PartialAttribute(name) => match self.next()? {
                Ok(Event::Value(value)) => {
                    Some(Ok(Event::Attribute(Attribute { key: name, value })))
                }
                Ok(_) => Some(Err(ReaderError::EXIP(EXIPError::Unexpected))),
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
                Ok(_) => Some(Err(ReaderError::EXIP(EXIPError::Unexpected))),
                Err(e) => Some(Err(e)),
            },
            _ => Some(Err(ReaderError::EXIP(EXIPError::Unexpected))),
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
        Err(_) => ffi::errorCode_EXIP_INVALID_EXI_INPUT,
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

unsafe extern "C" fn invoke_nsdec(
    ns: ffi::String,
    prefix: ffi::String,
    is_local: c_uint,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.namespace_declaration(
        from_stringtype(&ns as *const _).unwrap_or_default(),
        from_stringtype(&prefix as *const _).unwrap_or_default(),
        is_local != 0,
    ) {
        Ok(_) => ffi::errorCode_EXIP_OK,
        Err(e) => e as u32,
    }
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
        namespaceDeclaration: Some(invoke_nsdec),
        // EXIP never calls these functions
        warning: None,
        error: None,
        fatalError: None,
        processingInstruction: None,
        selfContained: None,
    }
}

#[test]
fn simple_read() {
    use crate::data::Name;

    let input = &[
        36, 69, 88, 73, 160, 65, 35, 67, 163, 163, 129, 209, 121, 123, 187, 187, 185, 115, 99, 163,
        169, 115, 155, 41, 122, 42, 74, 154, 98, 10, 17, 123, 155, 27, 67, 43, 107, 9, 107, 163,
        43, 155, 160, 138, 107, 171, 99, 163, 75, 131, 99, 42, 194, 154, 35, 154, 163, 43, 155,
        166, 144, 168, 208, 210, 230, 64, 210, 230, 64, 194, 220, 64, 202, 240, 194, 218, 224, 216,
        202, 64, 222, 204, 64, 230, 202, 228, 210, 194, 216, 210, 244, 210, 220, 206, 64, 138, 176,
        146, 64, 230, 232, 228, 202, 194, 218, 230, 64, 234, 230, 210, 220, 206, 64, 138, 176, 146,
        160, 64, 216, 222, 238, 64, 216, 202, 236, 202, 216, 64, 130, 160, 146,
    ];
    let mut reader = Reader::new(input.as_slice(), None, None).unwrap();
    assert_eq!(reader.next(), Some(Ok(Event::StartDocument)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
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
    let input = &[
        0x24, 0x45, 0x58, 0x49, 0xA0, 0x49, 0x6E, 0x05, 0x30, 0x2E, 0x32, 0x48, 0x54, 0x68, 0x69,
        0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x6E, 0x20, 0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65,
        0x20, 0x6F, 0x66, 0x20, 0x73, 0x65, 0x72, 0x69, 0x61, 0x6C, 0x69, 0x7A, 0x69, 0x6E, 0x67,
        0x20, 0x45, 0x58, 0x49, 0x20, 0x73, 0x74, 0x72, 0x65, 0x61, 0x6D, 0x73, 0x20, 0x75, 0x73,
        0x69, 0x6E, 0x67, 0x20, 0x45, 0x58, 0x49, 0x50, 0x20, 0x6C, 0x6F, 0x77, 0x20, 0x6C, 0x65,
        0x76, 0x65, 0x6C, 0x20, 0x41, 0x50, 0x49, 0x21, 0x2A, 0x34, 0x34, 0xB9, 0x90, 0x34, 0xB9,
        0x90, 0x30, 0x90, 0x3A, 0x32, 0xB9, 0xBA, 0x10, 0x37, 0xB3, 0x10, 0x38, 0x39, 0x37, 0xB1,
        0xB2, 0xB9, 0xB9, 0xB4, 0xB7, 0x33, 0x90, 0x2C, 0x26, 0xA6, 0x10, 0x39, 0xB1, 0xB4, 0x32,
        0xB6, 0xB2, 0xB9, 0x90, 0x3B, 0xB4, 0xBA, 0x34, 0x10, 0x36, 0xBA, 0xB6, 0x3A, 0x34, 0xB8,
        0x36, 0x32, 0x90, 0x2C, 0x29, 0xA2, 0x10, 0x33, 0x34, 0xB6, 0x32, 0xB9, 0x89, 0xD5, 0x99,
        0x5C, 0x9A, 0x59, 0x9E, 0x48, 0x1D, 0x1A, 0x18, 0x5D, 0x08, 0x1D, 0x1A, 0x19, 0x48, 0x1A,
        0x5B, 0x5C, 0x1B, 0x19, 0x5B, 0x59, 0x5B, 0x9D, 0x18, 0x5D, 0x1A, 0x5B, 0xDB, 0x88, 0x1D,
        0xDB, 0xDC, 0x9A, 0xDC, 0xC8, 0x4A, 0xD4, 0xDA, 0x5B, 0x5C, 0x1B, 0x19, 0x48, 0x1D, 0x19,
        0x5C, 0xDD, 0x08, 0x19, 0x5B, 0x19, 0x5B, 0x59, 0x5B, 0x9D, 0x08, 0x1D, 0xDA, 0x5D, 0x1A,
        0x08, 0x1C, 0xDA, 0x5B, 0x99, 0xDB, 0x19, 0x48, 0x18, 0x5D, 0x1D, 0x1C, 0x9A, 0x58, 0x9D,
        0x5D, 0x19, 0x47, 0x48, 0x3A, 0x8B, 0x06, 0x3F, 0xDB, 0x0E, 0xF9, 0x0A, 0x03, 0x90, 0x14,
        0x04, 0xDA, 0x5F, 0x4A, 0x41, 0xE4, 0xC3, 0x39, 0xDC, 0x1E, 0xC0,
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
    let mut reader = Reader::new(input.as_slice(), Some(schema), None).unwrap();
    assert_eq!(reader.next(), Some(Ok(Event::StartDocument)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "EXIPEncoder",
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
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
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
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
            namespace: Some("http://www.ltu.se/EISLAB/nested-xsd"),
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
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Integer(1001)))));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "bool",
            namespace: Some("http://www.ltu.se/EISLAB/nested-xsd"),
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
            namespace: Some("http://www.ltu.se/EISLAB/schema-test"),
            prefix: None
        })))
    );
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "byteTest",
            namespace: None,
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::Integer(11)))));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(
        reader.next(),
        Some(Ok(Event::StartElement(Name {
            local_name: "dateTimeTest",
            namespace: None,
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
            namespace: None,
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
            namespace: None,
            prefix: None
        })))
    );
    assert_eq!(reader.next(), Some(Ok(Event::Value(Value::String("hej")))));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndElement)));
    assert_eq!(reader.next(), Some(Ok(Event::EndDocument)));
}
