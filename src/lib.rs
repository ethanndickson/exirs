#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::CString;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// encodeTestEXI.c rewrite
#[test]
fn encodeTestEXI() {
    // EXI Data
    const NS_STR: &str = "http://www.ltu.se/EISLAB/schema-test";
    const NS_NESTED_STR: &str = "http://www.ltu.se/EISLAB/nested-xsd";
    const NS_TYPES_STR: &str = "http://www.ltu.se/EISLAB/types";
    const NS_EMPTY_STR: &str = "";
    const ELEM_ENCODE_STR: &str = "EXIPEncoder";
    const ELEM_MULT_TEST_STR: &str = "MultipleXSDsTest";
    const ELEM_DESCR_STR: &str = "description";
    const ELEM_TYPE_TEST_STR: &str = "type-test";
    const ELEM_TEST_SETUP_STR: &str = "testSetup";
    const ELEM_BOOL_STR: &str = "bool";
    const ELEM_INT_STR: &str = "int";
    const ELEM_EXT_TYPES_STR: &str = "extendedTypeTest";
    const ELEM_BYTE_TYPES_STR: &str = "byteTest";
    const ELEM_DATE_TYPES_STR: &str = "dateTimeTest";
    const ELEM_BIN_TYPES_STR: &str = "binaryTest";
    const ELEM_ENUM_TYPES_STR: &str = "enumTest";
    const ATTR_BYTE_STR: &str = "testByte";
    const ATTR_VERSION_STR: &str = "version";
    const ATTR_GOAL_STR: &str = "goal";
    const ATTR_ID_STR: &str = "id";
    const SOME_BINARY_DATA: [i32; 9] = [0x02, 0x2f, 0xa5, 0x20, 0xf2, 0x61, 0x9c, 0xee, 0x0f];
    const SOME_BINARY_DATA_BASE64: &str = "i3sd7fatzxad";
    const ENUM_DATA_4: &str = "hej";

    extern "C" {
        static stdout: *mut libc::FILE;
    }
    unsafe extern "C" fn writeFileOutputStream(
        buf: *mut ::std::os::raw::c_void,
        size: usize,
        stream: *mut ::std::os::raw::c_void,
    ) -> usize {
        let file: *mut libc::FILE = stream as *mut libc::FILE;
        return unsafe { libc::fwrite(buf, 1, size, file) };
    }

    unsafe {
        // test args
        let schemaPtr = std::ptr::null::<EXIPSchema>() as *mut _;
        let outfile = stdout;
        let outputStream = writeFileOutputStream;

        // Stream
        let mut testStrm: EXIStream = std::mem::zeroed();

        // EXI Type Class
        let mut valueType: EXITypeClass = 25;

        // Initialise buffer
        const OUTPUT_BUFFER_SIZE: usize = 200;
        let mut out_buf: Vec<i8> = vec![0; OUTPUT_BUFFER_SIZE];
        let mut buf: BinaryBuffer = std::mem::zeroed();
        buf.buf = out_buf.as_mut_ptr();
        buf.bufLen = OUTPUT_BUFFER_SIZE;

        // 1. Initialise the header of the stream
        (serialize.initHeader).unwrap()(&mut testStrm as *mut _);

        // 2. Set any options in the header
        testStrm.header.has_cookie = 1;
        testStrm.header.has_options = 1;
        testStrm.header.opts.valueMaxLength = 300;
        testStrm.header.opts.valuePartitionCapacity = 50;
        testStrm.header.opts.enumOpt |= 2; // set strict

        // 3. Define external stream for the output
        buf.ioStrm.readWriteToStream = Some(outputStream);
        buf.ioStrm.stream = outfile as *mut libc::c_void;

        // 4. Initialise Stream
        let ec = initStream(&mut testStrm as *mut _, buf, schemaPtr);
        assert_eq!(ec, 0);

        // 5. Start building
        let ec = (serialize.exiHeader).unwrap()(&mut testStrm as *mut _);
        assert_eq!(ec, 0);

        let ec = (serialize.startDocument).unwrap()(&mut testStrm as *mut _);
        assert_eq!(ec, 0);

        let mut qname: QName = std::mem::zeroed();
        let ns_str_c = CString::new(NS_STR).unwrap();
        qname.uri = &StringType {
            str_: ns_str_c.as_ptr() as *mut _,
            length: NS_STR.len(),
        };
        let elem_mult_test_str = CString::new(ELEM_MULT_TEST_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_mult_test_str.as_ptr() as *mut _,
            length: ELEM_MULT_TEST_STR.len(),
        };
        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let mut qname: QName = std::mem::zeroed();
        qname.uri = &StringType {
            str_: std::ptr::null::<std::os::raw::c_char>() as *mut std::os::raw::c_char,
            length: 0,
        };

        // let attrbytestr =
        // qname.localName = &StringType {}
    }
}
