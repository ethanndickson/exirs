use std::{
    mem::MaybeUninit,
    os::raw::{c_char, c_uint, c_void},
};

use crate::{
    data::{from_qname, from_stringtype, Event, Value},
    error::EXIPError,
};

#[derive(Default)]
struct Handler<'a> {
    latest: Option<Event<'a>>,
}

impl<'a> Handler<'a> {
    fn start_document(&mut self) -> Result<(), crate::error::EXIPError> {
        self.latest = Some(Event::StartDocument);
        Ok(())
    }

    fn end_document(&mut self) -> Result<(), crate::error::EXIPError> {
        self.latest = Some(Event::EndDocument);
        Ok(())
    }

    fn start_element(
        &mut self,
        name: crate::data::Name<'a>,
    ) -> Result<(), crate::error::EXIPError> {
        self.latest = Some(Event::StartElement(name));
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), crate::error::EXIPError> {
        self.latest = Some(Event::EndElement);
        Ok(())
    }

    fn attribute(&mut self, name: crate::data::Name) -> Result<(), crate::error::EXIPError> {
        todo!();
    }

    fn string(&mut self, value: &'a str) -> Result<(), crate::error::EXIPError> {
        self.latest = Some(Event::Value(Value::String(value)));
        Ok(())
    }

    fn decimal(&mut self, value: ffi::EXIFloat) -> Result<(), crate::error::EXIPError> {
        self.float(value)
    }

    fn boolean(&mut self, value: bool) -> Result<(), crate::error::EXIPError> {
        todo!();
    }

    fn datetime(&mut self, _: &str) -> Result<(), crate::error::EXIPError> {
        todo!();
    }

    fn binary(&mut self, _: &[u8], _: usize) -> Result<(), crate::error::EXIPError> {
        todo!();
    }

    fn qname(&mut self, _: crate::data::Name) -> Result<(), crate::error::EXIPError> {
        todo!();
    }

    fn int(&mut self, _: i64) -> Result<(), EXIPError> {
        todo!();
    }

    fn float(&mut self, value: ffi::EXIFloat) -> Result<(), EXIPError> {
        todo!();
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
    parser: Box<ffi::Parser>,
    _buf: Box<[u8]>,
    handler: Box<Handler<'a>>,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
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
        let ec =
            unsafe { (ffi::parse.setSchema).unwrap()(&mut parser as *mut _, std::ptr::null_mut()) };
        assert_eq!(ec, 0);
        Self {
            parser: Box::new(parser),
            _buf: heap_buf,
            handler,
        }
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
        // StartDocument is returned before parseNext is called
        if let Some(Event::StartDocument) = self.handler.latest {
            return Some(Ok(self.handler.latest.take().unwrap()));
        }
        // EndDocument is only returned once
        if let Some(Event::EndDocument) = self.handler.latest {
            return None;
        }
        let ec = unsafe { (ffi::parse.parseNext).unwrap()(self.parser.as_mut() as *mut _) };
        match ec {
            0 => Some(Ok(self.handler.latest.take().unwrap())),
            11 => Some(Ok(self.handler.latest.clone().unwrap())),
            e => Some(Err(e.into())),
        }
    }
}

unsafe extern "C" fn invoke_start_document(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.start_document() {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_end_document(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.end_document() {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_start_element(
    qname: ffi::QName,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.start_element(from_qname(qname)) {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_end_element(handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.end_element() {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_attribute(qname: ffi::QName, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_int(integer: ffi::Integer, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_boolean(bool: ffi::boolean, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_string(str: ffi::String, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    match handler.string(from_stringtype(&str as *const _).unwrap_or_default()) {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

unsafe extern "C" fn invoke_float(str: ffi::Float, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_binary(
    binary: *const c_char,
    nbytes: usize,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_datetime(
    dt_val: ffi::EXIPDateTime,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_decimal(dec_val: ffi::Decimal, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_list(
    exi_type: ffi::EXITypeClass,
    item_count: c_uint,
    handler: *mut c_void,
) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
}

unsafe extern "C" fn invoke_qname(qname: ffi::QName, handler: *mut c_void) -> ffi::errorCode {
    let handler = &mut *(handler as *mut Handler);
    0
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
    let mut reader = Reader::new(input);
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
