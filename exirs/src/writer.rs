use std::{mem::MaybeUninit, time::SystemTime};

use crate::{
    config::Header,
    data::{to_qname, to_stringtype, Attribute, Event, Name, NamespaceDeclaration, Value},
    error::EXIPError,
};

const OUTPUT_BUFFER_SIZE: usize = 8 * 1024;

pub struct Writer {
    uses_schema: bool,
    cur_tc: Box<ffi::EXITypeClass>,
    stream: Box<ffi::EXIStream>,
    _buf: Box<[i8; OUTPUT_BUFFER_SIZE]>,
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe { ffi::serialize.closeEXIStream.unwrap()(&mut *self.stream) };
    }
}

impl Writer {
    pub fn new(header: Header) -> Result<Self, EXIPError> {
        let mut stream: MaybeUninit<ffi::EXIStream> = MaybeUninit::uninit();
        unsafe { (ffi::serialize.initHeader).unwrap()(stream.as_mut_ptr()) };
        let ptr = stream.as_mut_ptr();
        header.apply_to_stream(ptr);
        let mut stream = unsafe { stream.assume_init() };

        let mut heap_buf = Box::new([0; OUTPUT_BUFFER_SIZE]); // 8KiB
        let buf = ffi::BinaryBuffer {
            buf: heap_buf.as_mut_ptr(),
            bufLen: OUTPUT_BUFFER_SIZE,
            bufContent: 0,
            ioStrm: ffi::ioStream {
                readWriteToStream: None,
                stream: std::ptr::null_mut(),
            },
        };
        let ec = unsafe { ffi::initStream(&mut stream as *mut _, buf, std::ptr::null_mut()) };
        if ec != 0 {
            return Err(ec.into());
        }
        let mut out = Self {
            stream: Box::new(stream),
            _buf: heap_buf,
            uses_schema: false,
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
            Event::Value(val) => self.value(val),
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

    fn value(&mut self, value: Value) -> Result<(), EXIPError> {
        if self.uses_schema {
            match value {
                Value::Integer(int) => self.integer(int),
                Value::Boolean(bool) => self.boolean(bool),
                Value::String(str) => self.characters(str),
                Value::Float(float) => self.float(float),
                Value::Binary(binary) => self.binary(binary),
                Value::Timestamp(ts) => self.timestamp(ts),
                Value::List(list) => self.list(list),
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
        unsafe {
            match ffi::serialize.startElement.unwrap()(
                self.stream.as_mut(),
                to_qname(name),
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
        let ec = unsafe {
            ffi::serialize.attribute.unwrap()(
                self.stream.as_mut(),
                to_qname(attr.key),
                true as u32,
                self.cur_tc.as_mut(),
            )
        };
        match ec {
            0 => Ok::<(), EXIPError>(()),
            e => Err(e.into()),
        }?;
        self.value(attr.value)
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
        todo!()
    }

    fn binary(&mut self, binary: &[u8]) -> Result<(), EXIPError> {
        todo!()
    }

    fn timestamp(&mut self, ts: &SystemTime) -> Result<(), EXIPError> {
        todo!()
    }

    fn list(&mut self, list: &[Value]) -> Result<(), EXIPError> {
        todo!()
    }

    fn type_value(&mut self, name: Name) -> Result<(), EXIPError> {
        let ec = unsafe {
            ffi::serialize.attribute.unwrap()(
                self.stream.as_mut(),
                to_qname(Name {
                    local_name: "type",
                    namespace: "http://www.w3.org/2001/XMLSchema-instance",
                    prefix: None,
                }),
                true as u32,
                self.cur_tc.as_mut(),
            )
        };
        match ec {
            0 => Ok::<(), EXIPError>(()),
            e => Err(e.into()),
        }?;
        unsafe {
            match ffi::serialize.qnameData.unwrap()(self.stream.as_mut(), to_qname(name)) {
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
}

impl Default for Writer {
    fn default() -> Self {
        // Default configuration should never fail
        Self::new(Header::default()).unwrap()
    }
}

#[test]
fn simple_write() {
    use crate::config::OptionFlags;

    let mut header = Header::default();
    header.has_cookie = true;
    header.has_options = true;
    header.opts.value_max_length = 300;
    header.opts.value_partition_capacity = 50;
    header.opts.flags.insert(OptionFlags::STRICT);
    let mut builder = Writer::new(header).unwrap();
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
        [
            36, 69, 88, 73, 160, 2, 172, 2, 12, 178, 18, 52, 58, 58, 56, 29, 23, 151, 187, 187,
            187, 151, 54, 58, 58, 151, 57, 178, 151, 162, 164, 169, 166, 32, 161, 23, 185, 177,
            180, 50, 182, 176, 150, 186, 50, 185, 186, 8, 166, 186, 182, 58, 52, 184, 54, 50, 172,
            41, 162, 57, 170, 50, 185, 186, 105, 10, 141, 13, 46, 100, 13, 46, 100, 12, 45, 196,
            12, 175, 12, 45, 174, 13, 140, 164, 13, 236, 196, 14, 108, 174, 77, 44, 45, 141, 47,
            77, 45, 204, 228, 8, 171, 9, 36, 14, 110, 142, 76, 172, 45, 174, 100, 14, 174, 109, 45,
            204, 228, 8, 171, 9, 42, 4, 13, 141, 238, 228, 13, 140, 174, 204, 173, 132, 8, 42, 9,
            32
        ],
        builder.get()
    )
}
