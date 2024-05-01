use std::mem::MaybeUninit;

use ffi::initStream;

use crate::{
    config::EXIHeader,
    data::{to_stringtype, Name, NamespaceDeclaration, SchemalessAttribute},
    error::EXIPError,
    events::SchemalessEvent,
};

const OUTPUT_BUFFER_SIZE: usize = 8 * 1024;

pub struct SchemalessBuilder {
    stream: Box<ffi::EXIStream>,
    _buf: Box<[i8; OUTPUT_BUFFER_SIZE]>,
}

impl Drop for SchemalessBuilder {
    fn drop(&mut self) {
        unsafe { ffi::serialize.closeEXIStream.unwrap()(&mut *self.stream) };
    }
}

impl SchemalessBuilder {
    pub fn new(header: EXIHeader) -> Self {
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
        let ec = unsafe { initStream(&mut stream as *mut _, buf, std::ptr::null_mut()) };
        assert_eq!(ec, 0);
        Self {
            stream: Box::new(stream),
            _buf: heap_buf,
        }
    }

    pub fn add(&mut self, event: SchemalessEvent) -> Result<(), EXIPError> {
        match event {
            SchemalessEvent::StartDocument => start_document(&mut self.stream),
            SchemalessEvent::EndDocument => end_document(&mut self.stream),
            SchemalessEvent::StartElement(name) => start_element(&mut self.stream, name),
            SchemalessEvent::EndElement => end_element(&mut self.stream),
            SchemalessEvent::Attribute(attr) => schemaless_attribute(&mut self.stream, attr),
            SchemalessEvent::Characters(str) => characters(&mut self.stream, str),
            SchemalessEvent::NamespaceDeclaration(ns) => namespace(&mut self.stream, ns),
            SchemalessEvent::ExiHeader => header(&mut self.stream),
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
}

impl Default for SchemalessBuilder {
    fn default() -> Self {
        Self::new(EXIHeader::default())
    }
}

fn start_document(stream: &mut ffi::EXIStream) -> Result<(), EXIPError> {
    unsafe {
        match ffi::serialize.startDocument.unwrap()(stream as *mut _) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn end_document(stream: &mut ffi::EXIStream) -> Result<(), EXIPError> {
    unsafe {
        match ffi::serialize.endDocument.unwrap()(stream as *mut _) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn start_element(stream: &mut ffi::EXIStream, name: Name) -> Result<(), EXIPError> {
    let qname = ffi::QName {
        uri: &to_stringtype(name.namespace),
        localName: &to_stringtype(name.local_name),
        prefix: match name.prefix {
            Some(n) => &to_stringtype(n),
            None => std::ptr::null(),
        },
    };
    let mut vt = 0;
    unsafe {
        match ffi::serialize.startElement.unwrap()(stream as *mut _, qname, &mut vt) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn end_element(stream: &mut ffi::EXIStream) -> Result<(), EXIPError> {
    unsafe {
        match ffi::serialize.endElement.unwrap()(stream as *mut _) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn schemaless_attribute(
    stream: &mut ffi::EXIStream,
    attr: SchemalessAttribute,
) -> Result<(), EXIPError> {
    let qname = ffi::QName {
        uri: &to_stringtype(attr.key.namespace),
        localName: &to_stringtype(attr.key.local_name),
        prefix: match attr.key.prefix {
            Some(n) => &to_stringtype(n),
            None => std::ptr::null(),
        },
    };
    let mut vt = 0;
    unsafe {
        match ffi::serialize.attribute.unwrap()(stream as *mut _, qname, 1, &mut vt) {
            0 => Ok::<(), EXIPError>(()),
            e => Err(e.into()),
        }?
    };
    // Get EXIP to allocate
    characters(stream, attr.value)
}

fn characters(stream: &mut ffi::EXIStream, characters: &str) -> Result<(), EXIPError> {
    unsafe {
        let chval = to_stringtype(characters);
        match ffi::serialize.stringData.unwrap()(stream as *mut _, chval) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn header(stream: &mut ffi::EXIStream) -> Result<(), EXIPError> {
    unsafe {
        match ffi::serialize.exiHeader.unwrap()(stream as *mut _) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

fn namespace(stream: &mut ffi::EXIStream, dec: NamespaceDeclaration) -> Result<(), EXIPError> {
    let ns = to_stringtype(dec.namespace);
    let prefix = to_stringtype(dec.prefix);
    unsafe {
        match ffi::serialize.namespaceDeclaration.unwrap()(
            stream,
            ns,
            prefix,
            dec.is_local_element as u32,
        ) {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
}

#[test]
fn simple() {
    use crate::config::EXIOptionFlags;

    let mut header = EXIHeader::default();
    header.has_cookie = true;
    header.has_options = true;
    header.opts.value_max_length = 300;
    header.opts.value_partition_capacity = 50;
    header.opts.flags.insert(EXIOptionFlags::STRICT);
    let mut builder = SchemalessBuilder::new(header);
    builder.add(SchemalessEvent::ExiHeader).unwrap();
    builder.add(SchemalessEvent::StartDocument).unwrap();
    builder
        .add(SchemalessEvent::StartElement(Name {
            local_name: "MultipleXSDsTest",
            namespace: "http://www.ltu.se/EISLAB/schema-test",
            prefix: None,
        }))
        .unwrap();
    builder
        .add(SchemalessEvent::Characters(
            "This is an example of serializing EXI streams using EXIP low level API",
        ))
        .unwrap();
    builder.add(SchemalessEvent::EndElement).unwrap();
    builder.add(SchemalessEvent::EndDocument).unwrap();
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
