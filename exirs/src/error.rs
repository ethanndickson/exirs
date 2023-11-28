pub enum EXIPError {
    NotImplemented = 1,
    Unexpected,
    HashTable,
    OutOfBounds,
    NullPointerRef,
    MemAlloc,
    InvalidHeader,
    InconsistentProcState,
    InvalidEXIInput,
    BufferEndReached,
    ParsingComplete,
    TooManyPrefxiesPerURI,
    InvalidEXIPConfig,
    NoPrefixesPreservedXMLSchema,
    InvalidStringOperation,
    HeaderOptionsMismatch,
    HandlerStop,
}

impl From<u32> for EXIPError {
    fn from(value: u32) -> Self {
        match value {
            1 => EXIPError::NotImplemented,
            2 => EXIPError::Unexpected,
            3 => EXIPError::HashTable,
            4 => EXIPError::OutOfBounds,
            5 => EXIPError::NullPointerRef,
            6 => EXIPError::MemAlloc,
            7 => EXIPError::InvalidHeader,
            8 => EXIPError::InconsistentProcState,
            9 => EXIPError::InvalidEXIInput,
            10 => EXIPError::BufferEndReached,
            11 => EXIPError::ParsingComplete,
            12 => EXIPError::TooManyPrefxiesPerURI,
            13 => EXIPError::InvalidEXIPConfig,
            14 => EXIPError::NoPrefixesPreservedXMLSchema,
            15 => EXIPError::InvalidStringOperation,
            16 => EXIPError::HeaderOptionsMismatch,
            17 => EXIPError::HandlerStop,
            _ => EXIPError::Unexpected,
        }
    }
}
