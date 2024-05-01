use bitflags::bitflags;
use ffi::EXIStream;

use crate::data::to_stringtype;

#[non_exhaustive]
pub struct EXIOptions {
    pub flags: EXIOptionFlags,
    pub preserve: PreservationFlags,
    pub schema_id_mode: SchemaIdMode,
    pub schema_id: Option<String>,
    pub blocksize: u32,
    pub value_max_length: usize,
    pub value_partition_capacity: usize,
}

impl Default for EXIOptions {
    fn default() -> Self {
        Self {
            flags: EXIOptionFlags::empty(),
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
    pub struct EXIOptionFlags: u8 {
        const ALIGNMENT = 0xc0;
        const COMPRESSION = 0x01;
        const STRICT = 0x02;
        const FRAGMENT = 0x04;
        const SELF_CONTAINED = 0x08;

    }
}

bitflags! {
    pub struct PreservationFlags: u8 {
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

pub struct EXIHeader {
    pub has_cookie: bool,
    pub has_options: bool,
    pub is_preview_version: bool,
    pub version_number: i16,
    pub opts: EXIOptions,
}

impl EXIHeader {
    pub(crate) fn apply_to_stream(self, ptr: *mut EXIStream) {
        unsafe {
            (*ptr).header.has_cookie = self.has_cookie as u32;
            (*ptr).header.has_options = self.has_options as u32;
            (*ptr).header.is_preview_version = self.is_preview_version as u32;
            (*ptr).header.version_number = self.version_number;
            (*ptr).header.opts.enumOpt = self.opts.flags.bits();
            (*ptr).header.opts.preserve = self.opts.preserve.bits();
            (*ptr).header.opts.schemaIDMode = self.opts.schema_id_mode as u32;
            // We repesent no schema ID as None, exip expects empty string
            (*ptr).header.opts.schemaID =
                to_stringtype(self.opts.schema_id.unwrap_or("".to_owned()).as_str());
            (*ptr).header.opts.blockSize = self.opts.blocksize;
            (*ptr).header.opts.valueMaxLength = self.opts.value_max_length;
            (*ptr).header.opts.valuePartitionCapacity = self.opts.value_partition_capacity;
            // drMap and user_defined_data left default as exip never touches them
        }
    }
}

impl Default for EXIHeader {
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
