use std::mem::MaybeUninit;

use bitflags::bitflags;

use crate::{data::to_stringtype, error::SchemaError};

#[non_exhaustive]
pub struct Options<'a> {
    flags: OptionFlags,
    preserve: PreservationFlags,
    schema_id_mode: SchemaIdMode,
    schema_id: Option<&'a str>,
    blocksize: u32,
    value_max_length: usize,
    value_partition_capacity: usize,
}

pub enum Alignment {
    BitPacked = 0,
    ByteAlignment = 1,
    PreCompression = 2,
}

impl From<Alignment> for OptionFlags {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::BitPacked => OptionFlags::BIT_PACKED,
            Alignment::ByteAlignment => OptionFlags::BYTE_ALIGNMENT,
            Alignment::PreCompression => OptionFlags::PRE_COMPRESSION,
        }
    }
}

impl<'a> Options<'a> {
    pub(crate) fn ffi(self) -> ffi::EXIOptions {
        ffi::EXIOptions {
            enumOpt: self.flags.bits(),
            preserve: self.preserve.bits(),
            schemaIDMode: self.schema_id_mode as u32,
            // We repesent no schema ID as Option::None, exip wants an empty string
            schemaID: to_stringtype(self.schema_id.unwrap_or("")),
            blockSize: self.blocksize,
            valueMaxLength: self.value_max_length,
            valuePartitionCapacity: self.value_partition_capacity,
            // drMap and user_defined_data left null as exip never touches them
            user_defined_data: std::ptr::null_mut(),
            drMap: std::ptr::null_mut(),
        }
    }

    pub fn new() -> Options<'a> {
        Self::default()
    }

    pub fn strict(mut self, is: bool) -> Self {
        self.flags.set(OptionFlags::STRICT, is);
        self
    }

    pub fn fragment(mut self, is: bool) -> Self {
        self.flags.set(OptionFlags::FRAGMENT, is);
        self
    }

    pub fn compression(mut self, is: bool) -> Self {
        self.flags.set(OptionFlags::COMPRESSION, is);
        self
    }

    pub fn self_contained(mut self, is: bool) -> Self {
        self.flags.set(OptionFlags::SELF_CONTAINED, is);
        self
    }

    pub fn alignment(mut self, val: Alignment) -> Self {
        self.flags &= OptionFlags::RESET_ALIGNMENT;
        self.flags |= val.into();
        self
    }

    pub fn preserve_comments(mut self, is: bool) -> Self {
        self.preserve.set(PreservationFlags::COMMENTS, is);
        self
    }

    pub fn preserve_processing_instructions(mut self, is: bool) -> Self {
        self.preserve.set(PreservationFlags::PIS, is);
        self
    }

    pub fn preserve_dt_and_er(mut self, is: bool) -> Self {
        self.preserve.set(PreservationFlags::DTD, is);
        self
    }

    pub fn preserve_prefixes(mut self, is: bool) -> Self {
        self.preserve.set(PreservationFlags::PREFIXES, is);
        self
    }

    pub fn preserve_lexical_values(mut self, is: bool) -> Self {
        self.preserve.set(PreservationFlags::LEXVALUES, is);
        self
    }

    pub fn schema_id_mode(mut self, mode: SchemaIdMode) -> Self {
        self.schema_id_mode = mode;
        self
    }

    pub fn schema_id(mut self, id: &'a str) -> Self {
        self.schema_id = Some(id);
        self
    }

    pub fn blocksize(mut self, blocksize: u32) -> Self {
        self.blocksize = blocksize;
        self
    }
}

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        Self {
            flags: OptionFlags::empty(),
            preserve: PreservationFlags::empty(),
            schema_id_mode: SchemaIdMode::Absent,
            schema_id: None,
            blocksize: 1_000_000,
            // usize::MAX -> EXIP unbounded
            value_max_length: usize::MAX,
            value_partition_capacity: usize::MAX,
        }
    }
}

bitflags! {
    struct OptionFlags: u8 {
        const RESET_ALIGNMENT = 0b00111111;
        const ALIGNMENT = 0xc0;
        const BIT_PACKED = 0x00;
        const BYTE_ALIGNMENT = 0x40;
        const PRE_COMPRESSION = 0x80;
        const COMPRESSION = 0x01;
        const STRICT = 0x02;
        const FRAGMENT = 0x04;
        const SELF_CONTAINED = 0x08;

    }
}

bitflags! {
    struct PreservationFlags: u8 {
        const COMMENTS = 0x01;
        const PIS = 0x02;
        const DTD = 0x04;
        const PREFIXES = 0x08;
        const LEXVALUES = 0x10;
    }
}

#[repr(u8)]
pub enum SchemaIdMode {
    Absent,
    Set,
    Nil,
    Empty,
}

pub struct Header<'a> {
    has_cookie: bool,
    has_options: bool,
    is_preview_version: bool,
    version_number: i16,
    opts: Options<'a>,
}

impl<'a> Header<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(opts: Options<'a>) -> Self {
        Self {
            opts,
            has_options: true,
            ..Default::default()
        }
    }

    pub fn has_cookie(mut self, cookie: bool) -> Self {
        self.has_cookie = cookie;
        self
    }

    pub fn is_preview_version(mut self, is: bool) -> Self {
        self.is_preview_version = is;
        self
    }

    pub fn set_version_number(mut self, n: i16) -> Self {
        self.version_number = n;
        self
    }

    pub(crate) fn apply(self, ptr: *mut ffi::EXIStream) {
        unsafe {
            (*ptr).header.has_cookie = self.has_cookie as u32;
            (*ptr).header.has_options = self.has_options as u32;
            (*ptr).header.is_preview_version = self.is_preview_version as u32;
            (*ptr).header.version_number = self.version_number;
            (*ptr).header.opts = self.opts.ffi();
        }
    }
}

impl<'a> Default for Header<'a> {
    fn default() -> Self {
        Self {
            has_cookie: false,
            has_options: false,
            is_preview_version: false,
            version_number: 1,
            opts: Default::default(),
        }
    }
}

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
                opts.map_or(std::ptr::null_mut(), |opts| &mut opts.ffi()),
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
