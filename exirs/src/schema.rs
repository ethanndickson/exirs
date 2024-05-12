use std::mem::MaybeUninit;

use crate::{config::Options, error::SchemaError};

pub struct Schema {
    pub(crate) inner: Box<ffi::EXIPSchema>,
}

impl Schema {
    /// EXIP sets an arbitrary(?) maximum in it's examples
    /// I assume its just a memory concern?
    const MAX_XSD_FILES_COUNT: usize = 10;

    pub fn new<'a>(paths: &'a [&'a str], opts: Option<Options>) -> Result<Schema, SchemaError<'a>> {
        let num_files = paths.len();

        if num_files > Self::MAX_XSD_FILES_COUNT {
            return Err(SchemaError::TooManyXsds);
        }

        let mut heap_bufs = Vec::with_capacity(num_files);
        let mut buf_reps = Vec::with_capacity(num_files);
        for path in paths {
            let mut bytes = std::fs::read(path).map_err(|_| SchemaError::BadFile(path))?;
            let buf_rep = ffi::BinaryBuffer {
                buf: bytes.as_mut_ptr() as *mut _,
                bufLen: bytes.len(),
                bufContent: bytes.len(),
                ioStrm: ffi::ioStream {
                    readWriteToStream: None,
                    stream: std::ptr::null_mut(),
                },
            };
            heap_bufs.push(bytes);
            buf_reps.push(buf_rep);
        }

        let mut schema: MaybeUninit<ffi::EXIPSchema> = MaybeUninit::uninit();
        let ec = unsafe {
            ffi::generateSchemaInformedGrammars(
                buf_reps.as_mut_ptr(),
                num_files as u32,
                ffi::SchemaFormat_SCHEMA_FORMAT_XSD_EXI,
                opts.map_or(std::ptr::null_mut(), |opts| &mut opts.to_raw()),
                schema.as_mut_ptr(),
                None,
            )
        };
        if ec != 0 {
            return Err(SchemaError::GramGenFail);
        }
        let schema = unsafe { schema.assume_init() };
        Ok(Self {
            inner: Box::new(schema),
        })
    }
}
