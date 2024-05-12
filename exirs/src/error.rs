#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum EXIPError {
    #[error("unimplemented in EXIP")]
    NotImplemented = 1,
    #[error("unexpected error within EXIP")]
    Unexpected = 2,
    #[error("EXIP internal hash table error")]
    HashTable = 3,
    #[error("array out of bounds")]
    OutOfBounds = 4,
    #[error("attempted null pointer dereferenced")]
    NullPointerRef = 5,
    #[error("EXIP internal memory allocation failure")]
    MemAlloc = 6,
    #[error("invalid EXI Header")]
    InvalidHeader = 7,
    #[error("EXIP processor inconsistent with stream events")]
    InconsistentProcState = 8,
    #[error("received invalid EXI value or type encoding")]
    InvalidEXIInput = 9,
    #[error("the end of the available buffer was reached")]
    BufferEndReached = 10,
    #[error("EXIP was compiled with less than the required number of URI prefixes")]
    TooManyPrefxiesPerURI = 12,
    #[error("an invalid configuration was supplied to EXIP")]
    InvalidEXIPConfig = 13,
    #[error("XML Schema must be EXI encoded with the prefixes preserved")]
    NoPrefixesPreservedXMLSchema = 14,
    #[error("invalid String Operation")]
    InvalidStringOperation = 15,
    #[error("mismatch in the supplied header options")]
    HeaderOptionsMismatch = 16,
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
            12 => EXIPError::TooManyPrefxiesPerURI,
            13 => EXIPError::InvalidEXIPConfig,
            14 => EXIPError::NoPrefixesPreservedXMLSchema,
            15 => EXIPError::InvalidStringOperation,
            16 => EXIPError::HeaderOptionsMismatch,
            _ => EXIPError::Unexpected,
        }
    }
}

impl From<EXIPError> for u32 {
    fn from(value: EXIPError) -> Self {
        value as u32
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum SchemaError<'a> {
    #[error("too many file paths were supplied")]
    TooManyXsds,
    #[error("could not open file `{0}`")]
    BadFile(&'a str),
    #[error("EXIP could not allocate the required memory")]
    MallocFail,
    #[error("failed generating grammars")]
    GramGenFail,
}
