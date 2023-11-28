use std::{io::Write, mem::MaybeUninit};

use ffi::initStream;

use crate::{error::EXIPError, events::SchemalessEvent};

pub struct SchemalessBuilder {
    stream: ffi::EXIStream,
}

impl SchemalessBuilder {
    const OUTPUT_BUFFER_SIZE: usize = 8 * 1024;

    pub fn new() -> Self {
        unsafe {
            let mut stream: MaybeUninit<ffi::EXIStream> = MaybeUninit::zeroed();
            (ffi::serialize.initHeader).unwrap()(stream.as_mut_ptr());
            let ptr = stream.as_mut_ptr();
            (*ptr).header.has_cookie = 1;
            (*ptr).header.has_options = 1;
            (*ptr).header.opts.valueMaxLength = 300;
            (*ptr).header.opts.valuePartitionCapacity = 50;
            (*ptr).header.opts.enumOpt |= 2; // set strict
            let mut stream = stream.assume_init();

            let mut heap_buf = vec![0; Self::OUTPUT_BUFFER_SIZE]; // 8KiB
            let mut buf: MaybeUninit<ffi::BinaryBuffer> = MaybeUninit::zeroed();
            let ptr = buf.as_mut_ptr();
            (*ptr).buf = heap_buf.as_mut_ptr();
            (*ptr).bufLen = Self::OUTPUT_BUFFER_SIZE;
            (*ptr).ioStrm.stream = std::ptr::null_mut();
            (*ptr).ioStrm.readWriteToStream = None;
            std::mem::forget(heap_buf);
            let buf = buf.assume_init();

            let ec = initStream(&mut stream as *mut _, buf, std::ptr::null_mut());
            assert_eq!(ec, 0);
            Self { stream: stream }
        }
    }

    pub fn add(&mut self, event: SchemalessEvent) -> Result<(), EXIPError> {
        match event {
            SchemalessEvent::StartDocument => start_document(&mut self.stream),
            SchemalessEvent::EndDocument => end_document(&mut self.stream),
            SchemalessEvent::StartElement(_) => todo!(),
            SchemalessEvent::EndElement => todo!(),
            SchemalessEvent::Attribute(_) => todo!(),
            SchemalessEvent::NamespaceDeclaration {
                namespace,
                prefix,
                is_local,
            } => todo!(),
            SchemalessEvent::ExiHeader => todo!(),
            SchemalessEvent::SelfContained => todo!(),
        }
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
