use std::mem::MaybeUninit;

use bytes::Bytes;

use crate::{
    config::{Header, Options, Schema},
    data::{to_stringtype, Attribute, Event, Name, NamespaceDeclaration, Value},
    error::EXIPError,
    to_qname,
};

const OUTPUT_BUFFER_SIZE: usize = 8 * 1024;

pub struct Writer {
    uses_schema: bool,
    cur_tc: Box<ffi::EXITypeClass>,
    stream: Box<ffi::EXIStream>,
    _buf: Box<[u8; OUTPUT_BUFFER_SIZE]>,
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe { ffi::serialize.closeEXIStream.unwrap()(&mut *self.stream) };
    }
}

impl Writer {
    pub fn new(header: Header, schema: Option<Schema>) -> Result<Self, EXIPError> {
        let uses_schema = schema.is_some();
        let mut stream: MaybeUninit<ffi::EXIStream> = MaybeUninit::uninit();
        unsafe { (ffi::serialize.initHeader).unwrap()(stream.as_mut_ptr()) };
        let ptr = stream.as_mut_ptr();
        header.apply(ptr);
        let mut stream = unsafe { stream.assume_init() };

        let mut heap_buf = Box::new([0_u8; OUTPUT_BUFFER_SIZE]); // 8KiB
        let buf = ffi::BinaryBuffer {
            buf: heap_buf.as_mut_ptr() as *mut i8,
            bufLen: OUTPUT_BUFFER_SIZE,
            bufContent: 0,
            ioStrm: ffi::ioStream {
                readWriteToStream: None,
                stream: std::ptr::null_mut(),
            },
        };
        let ec = unsafe {
            ffi::initStream(
                &mut stream as *mut _,
                buf,
                schema.map_or(std::ptr::null_mut(), |mut s| s.inner.as_mut()),
            )
        };
        if ec != 0 {
            return Err(ec.into());
        }
        let mut out = Self {
            stream: Box::new(stream),
            _buf: heap_buf,
            uses_schema,
            // Doesn't get read before it's written to by EXIP
            cur_tc: Box::new(0),
        };
        let ec = unsafe { ffi::serialize.exiHeader.unwrap()(out.stream.as_mut()) };
        if ec != 0 {
            return Err(ec.into());
        }
        Ok(out)
    }

    pub fn add(&mut self, event: Event) -> Result<(), EXIPError> {
        match event {
            Event::StartDocument => self.start_document(),
            Event::EndDocument => self.end_document(),
            Event::StartElement(name) => self.start_element(name),
            Event::EndElement => self.end_element(),
            Event::Attribute(attr) => self.attribute(attr),
            Event::Value(val) => self.value(&val),
            Event::NamespaceDeclaration(ns) => self.namespace(ns),
            Event::TypeAttribute(name) => self.type_value(name),
        }
    }

    pub fn get(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.stream.buffer.buf as *mut u8,
                self.stream.buffer.bufContent,
            )
        }
    }

    fn value(&mut self, value: &Value) -> Result<(), EXIPError> {
        if self.uses_schema {
            match value {
                Value::Integer(int) => self.integer(*int),
                Value::Boolean(bool) => self.boolean(*bool),
                Value::String(str) => self.characters(str),
                Value::Float(float) => self.float(*float),
                Value::Binary(binary) => self.binary(binary),
                Value::Timestamp(ts) => self.timestamp(ts),
                Value::List(list) => self.list(list),
                Value::QName(qname) => self.qname(qname),
            }
        } else {
            match value {
                Value::String(str) => self.characters(str),
                other => self.characters(&other.to_string()),
            }
        }
    }

    fn start_document(&mut self) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.startDocument.unwrap()(self.stream.as_mut()) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn end_document(&mut self) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.endDocument.unwrap()(self.stream.as_mut()) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn start_element(&mut self, name: Name) -> Result<(), EXIPError> {
        let qname = to_qname!(name);
        unsafe {
            match ffi::serialize.startElement.unwrap()(
                self.stream.as_mut(),
                qname,
                self.cur_tc.as_mut(),
            ) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn end_element(&mut self) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.endElement.unwrap()(self.stream.as_mut()) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn attribute(&mut self, attr: Attribute) -> Result<(), EXIPError> {
        // Inlined to keep the StringTypes in scope
        let qname = to_qname!(attr.key);
        let ec = unsafe {
            ffi::serialize.attribute.unwrap()(
                self.stream.as_mut(),
                qname,
                true as u32,
                self.cur_tc.as_mut(),
            )
        };
        match ec {
            0 => Ok::<(), EXIPError>(()),
            e => Err(e.into()),
        }?;
        self.value(&attr.value)
    }

    fn integer(&mut self, int: i64) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.intData.unwrap()(self.stream.as_mut(), int) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn boolean(&mut self, bool: bool) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.booleanData.unwrap()(self.stream.as_mut(), bool as u32) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn characters(&mut self, characters: &str) -> Result<(), EXIPError> {
        unsafe {
            let chval = to_stringtype(characters);
            match ffi::serialize.stringData.unwrap()(self.stream.as_mut(), chval) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn float(&mut self, float: f64) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.floatData.unwrap()(self.stream.as_mut(), float.into()) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn binary(&mut self, binary: &[u8]) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.binaryData.unwrap()(
                self.stream.as_mut(),
                binary.as_ptr() as *const _,
                binary.len(),
            ) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn timestamp(&mut self, ts: &chrono::NaiveDateTime) -> Result<(), EXIPError> {
        let dt: ffi::EXIPDateTime = ts.try_into().map_err(|_| EXIPError::InvalidEXIInput)?;
        unsafe {
            match ffi::serialize.dateTimeData.unwrap()(self.stream.as_mut(), dt) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn list(&mut self, list: &[Value]) -> Result<(), EXIPError> {
        unsafe {
            match ffi::serialize.listData.unwrap()(
                self.stream.as_mut(),
                list.len()
                    .try_into()
                    .map_err(|_| EXIPError::InvalidEXIInput)?,
            ) {
                0 => Ok::<(), EXIPError>(()),
                e => Err(e.into()),
            }
        }?;
        for each in list {
            self.value(each)?;
        }
        Ok(())
    }

    fn type_value(&mut self, name: Name) -> Result<(), EXIPError> {
        let typename = Name {
            local_name: "type",
            namespace: "http://www.w3.org/2001/XMLSchema-instance",
            prefix: None,
        };
        let qname = to_qname!(typename);
        let ec = unsafe {
            ffi::serialize.attribute.unwrap()(
                self.stream.as_mut(),
                qname,
                true as u32,
                self.cur_tc.as_mut(),
            )
        };
        match ec {
            0 => Ok::<(), EXIPError>(()),
            e => Err(e.into()),
        }?;
        let qname = to_qname!(name);
        unsafe {
            match ffi::serialize.qnameData.unwrap()(self.stream.as_mut(), qname) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn namespace(&mut self, dec: NamespaceDeclaration) -> Result<(), EXIPError> {
        let ns = to_stringtype(dec.namespace);
        let prefix = to_stringtype(dec.prefix);
        unsafe {
            match ffi::serialize.namespaceDeclaration.unwrap()(
                self.stream.as_mut(),
                ns,
                prefix,
                dec.is_local_element as u32,
            ) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }

    fn qname(&mut self, name: &Name) -> Result<(), EXIPError> {
        let qname = to_qname!(name);
        unsafe {
            match ffi::serialize.qnameData.unwrap()(self.stream.as_mut(), qname) {
                0 => Ok(()),
                e => Err(e.into()),
            }
        }
    }
}

impl Default for Writer {
    fn default() -> Self {
        // Default configuration should never fail
        Self::new(Header::default(), None).unwrap()
    }
}

#[test]
fn simple_schemaless_write() {
    let options = Options::default().strict(true);
    let header = Header::with_options(options).has_cookie(true);
    let mut builder = Writer::new(header, None).unwrap();
    builder.add(Event::StartDocument).unwrap();
    builder
        .add(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap();
    builder
        .add(Event::Value(Value::String(
            "This is an example of serializing EXI streams using EXIP low level API",
        )))
        .unwrap();
    builder.add(Event::EndElement).unwrap();
    builder.add(Event::EndDocument).unwrap();
    assert_eq!(
        builder.get(),
        [
            36, 69, 88, 73, 160, 65, 35, 67, 163, 163, 129, 209, 121, 123, 187, 187, 185, 115, 99,
            163, 169, 115, 155, 41, 122, 42, 74, 154, 98, 10, 17, 123, 155, 27, 67, 43, 107, 9,
            107, 163, 43, 155, 160, 138, 107, 171, 99, 163, 75, 131, 99, 42, 194, 154, 35, 154,
            163, 43, 155, 166, 144, 168, 208, 210, 230, 64, 210, 230, 64, 194, 220, 64, 202, 240,
            194, 218, 224, 216, 202, 64, 222, 204, 64, 230, 202, 228, 210, 194, 216, 210, 244, 210,
            220, 206, 64, 138, 176, 146, 64, 230, 232, 228, 202, 194, 218, 230, 64, 234, 230, 210,
            220, 206, 64, 138, 176, 146, 160, 64, 216, 222, 238, 64, 216, 202, 236, 202, 216, 64,
            130, 160, 146
        ]
    )
}

#[test]
fn full_write() {
    use chrono::NaiveDateTime;

    let options = Options::default().strict(true);
    let header = Header::with_options(options).has_cookie(true);
    let schema = Schema::new(
        &[
            "./examples/exipe-test-xsd.exi",
            "./examples/exipe-test-types-xsd.exi",
            "./examples/exipe-test-nested-xsd.exi",
        ],
        None,
    )
    .unwrap();
    let mut builder = Writer::new(header, Some(schema)).unwrap();
    builder.add(Event::StartDocument).unwrap();
    builder
        .add(Event::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap(); // <MultipleXSDsTest>
    builder
        .add(Event::StartElement(Name {
            local_name: "EXIPEncoder",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap(); // <EXIPEncoder>
    builder
        .add(Event::Attribute(Attribute {
            key: Name {
                local_name: "testByte",
                namespace: "",
                prefix: None,
            },
            value: Value::Integer(55),
        }))
        .unwrap(); // testByte=55
    builder
        .add(Event::Attribute(Attribute {
            key: Name {
                local_name: "version",
                namespace: "",
                prefix: None,
            },
            value: Value::String("0.2"),
        }))
        .unwrap(); // version="0.2"
    builder
        .add(Event::Value(Value::String(
            "This is an example of serializing EXI streams using EXIP low level API",
        )))
        .unwrap();
    builder.add(Event::EndElement).unwrap(); // </EXIPEncoder>
    builder
        .add(Event::StartElement(Name {
            local_name: "description",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap(); // <description>
    builder
        .add(Event::Value(Value::String(
            "This is a test of processing XML schemes with multiple XSD files",
        )))
        .unwrap();
    builder.add(Event::EndElement).unwrap(); // </description>
    builder
        .add(Event::StartElement(Name {
            local_name: "testSetup",
            namespace: "http://www.ltu.se/EISLAB/nested-xsd",
            prefix: None,
        }))
        .unwrap(); // <testSetup>
    builder
        .add(Event::Attribute(Attribute {
            key: Name {
                local_name: "goal",
                namespace: "",
                prefix: None,
            },
            value: Value::String("Verify that the implementation works!"),
        }))
        .unwrap(); // goal="Verify that the implementation works!"
    builder
        .add(Event::Value(Value::String(
            "Simple test element with single attribute",
        )))
        .unwrap();
    builder.add(Event::EndElement).unwrap(); // </testSetup>
    builder
        .add(Event::StartElement(Name {
            local_name: "type-test",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap(); // <type-test>
    builder
        .add(Event::Attribute(Attribute {
            key: Name {
                local_name: "id",
                namespace: "",
                prefix: None,
            },
            value: Value::Integer(1001),
        }))
        .unwrap(); // id=1001
    builder
        .add(Event::StartElement(Name {
            local_name: "bool",
            namespace: "http://www.ltu.se/EISLAB/nested-xsd",
            prefix: None,
        }))
        .unwrap(); // <bool>
    builder.add(Event::Value(Value::Boolean(true))).unwrap();
    builder.add(Event::EndElement).unwrap(); // </bool>
    builder.add(Event::EndElement).unwrap(); // </type-test>
    builder
        .add(Event::StartElement(Name {
            local_name: "extendedTypeTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap(); // <extendedTypeTest>
    builder
        .add(Event::StartElement(Name {
            local_name: "byteTest",
            namespace: "",
            prefix: None,
        }))
        .unwrap(); // <byteTest>
    builder.add(Event::Value(Value::Integer(11))).unwrap();
    builder.add(Event::EndElement).unwrap();
    builder
        .add(Event::StartElement(Name {
            local_name: "dateTimeTest",
            namespace: "",
            prefix: None,
        }))
        .unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2012, 7, 31).unwrap();
    let time = chrono::NaiveTime::from_hms_micro_opt(13, 33, 55, 839).unwrap();
    builder
        .add(Event::Value(Value::Timestamp(&NaiveDateTime::new(
            date, time,
        ))))
        .unwrap();
    builder.add(Event::EndElement).unwrap();
    builder
        .add(Event::StartElement(Name {
            local_name: "binaryTest",
            namespace: "",
            prefix: None,
        }))
        .unwrap();
    builder
        .add(Event::Value(Value::Binary(Bytes::from_static(&[
            0x02, 0x6d, 0x2f, 0xa5, 0x20, 0xf2, 0x61, 0x9c, 0xee, 0x0f,
        ]))))
        .unwrap();
    builder.add(Event::EndElement).unwrap(); // </binaryTest>
    builder
        .add(Event::StartElement(Name {
            local_name: "enumTest",
            namespace: "",
            prefix: None,
        }))
        .unwrap();
    builder.add(Event::Value(Value::String("hej"))).unwrap();
    builder.add(Event::EndElement).unwrap();
    builder.add(Event::EndElement).unwrap();
    builder.add(Event::EndElement).unwrap();
    builder.add(Event::EndDocument).unwrap();
    // From EXIP output
    assert_eq!(
        builder.get(),
        [
            0x24, 0x45, 0x58, 0x49, 0xA0, 0x49, 0x6E, 0x05, 0x30, 0x2E, 0x32, 0x48, 0x54, 0x68,
            0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x6E, 0x20, 0x65, 0x78, 0x61, 0x6D, 0x70,
            0x6C, 0x65, 0x20, 0x6F, 0x66, 0x20, 0x73, 0x65, 0x72, 0x69, 0x61, 0x6C, 0x69, 0x7A,
            0x69, 0x6E, 0x67, 0x20, 0x45, 0x58, 0x49, 0x20, 0x73, 0x74, 0x72, 0x65, 0x61, 0x6D,
            0x73, 0x20, 0x75, 0x73, 0x69, 0x6E, 0x67, 0x20, 0x45, 0x58, 0x49, 0x50, 0x20, 0x6C,
            0x6F, 0x77, 0x20, 0x6C, 0x65, 0x76, 0x65, 0x6C, 0x20, 0x41, 0x50, 0x49, 0x21, 0x2A,
            0x34, 0x34, 0xB9, 0x90, 0x34, 0xB9, 0x90, 0x30, 0x90, 0x3A, 0x32, 0xB9, 0xBA, 0x10,
            0x37, 0xB3, 0x10, 0x38, 0x39, 0x37, 0xB1, 0xB2, 0xB9, 0xB9, 0xB4, 0xB7, 0x33, 0x90,
            0x2C, 0x26, 0xA6, 0x10, 0x39, 0xB1, 0xB4, 0x32, 0xB6, 0xB2, 0xB9, 0x90, 0x3B, 0xB4,
            0xBA, 0x34, 0x10, 0x36, 0xBA, 0xB6, 0x3A, 0x34, 0xB8, 0x36, 0x32, 0x90, 0x2C, 0x29,
            0xA2, 0x10, 0x33, 0x34, 0xB6, 0x32, 0xB9, 0x89, 0xD5, 0x99, 0x5C, 0x9A, 0x59, 0x9E,
            0x48, 0x1D, 0x1A, 0x18, 0x5D, 0x08, 0x1D, 0x1A, 0x19, 0x48, 0x1A, 0x5B, 0x5C, 0x1B,
            0x19, 0x5B, 0x59, 0x5B, 0x9D, 0x18, 0x5D, 0x1A, 0x5B, 0xDB, 0x88, 0x1D, 0xDB, 0xDC,
            0x9A, 0xDC, 0xC8, 0x4A, 0xD4, 0xDA, 0x5B, 0x5C, 0x1B, 0x19, 0x48, 0x1D, 0x19, 0x5C,
            0xDD, 0x08, 0x19, 0x5B, 0x19, 0x5B, 0x59, 0x5B, 0x9D, 0x08, 0x1D, 0xDA, 0x5D, 0x1A,
            0x08, 0x1C, 0xDA, 0x5B, 0x99, 0xDB, 0x19, 0x48, 0x18, 0x5D, 0x1D, 0x1C, 0x9A, 0x58,
            0x9D, 0x5D, 0x19, 0x47, 0x48, 0x3A, 0x8B, 0x06, 0x3F, 0xDB, 0x0E, 0xF9, 0x0A, 0x03,
            0x90, 0x14, 0x04, 0xDA, 0x5F, 0x4A, 0x41, 0xE4, 0xC3, 0x39, 0xDC, 0x1E, 0xC0,
        ]
    );
}
