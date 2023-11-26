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
    const ELEM_ENCODE_STR: &str = "EXIPEncoder";
    const ELEM_MULT_TEST_STR: &str = "MultipleXSDsTest";
    const ELEM_DESCR_STR: &str = "description";
    const ELEM_TYPE_TEST_STR: &str = "type-test";
    const ELEM_TEST_SETUP_STR: &str = "testSetup";
    const ELEM_BOOL_STR: &str = "bool";
    const ELEM_EXT_TYPES_STR: &str = "extendedTypeTest";
    const ELEM_BYTE_TYPES_STR: &str = "byteTest";
    const ELEM_DATE_TYPES_STR: &str = "dateTimeTest";
    const ELEM_BIN_TYPES_STR: &str = "binaryTest";
    const ELEM_ENUM_TYPES_STR: &str = "enumTest";
    const ATTR_BYTE_STR: &str = "testByte";
    const ATTR_VERSION_STR: &str = "version";
    const ATTR_GOAL_STR: &str = "goal";
    const ATTR_ID_STR: &str = "id";
    const SOME_BINARY_DATA_BASE64: &str = "i3sd7fatzxad";
    const ENUM_DATA_4: &str = "hej";
    const nullstr_c: StringType = StringType {
        str_: std::ptr::null::<std::os::raw::c_char>() as *mut _,
        length: 0,
    };

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
        const OUTPUT_BUFFER_SIZE: usize = 519;
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
        let ns_str_c = StringType {
            str_: ns_str_c.as_ptr() as *mut _,
            length: NS_STR.len(),
        };
        let elem_mult_test_str = CString::new(ELEM_MULT_TEST_STR).unwrap();
        qname.uri = &ns_str_c;
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

        qname.uri = &ns_str_c;
        let elem_encode_str = CString::new(ELEM_ENCODE_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_encode_str.as_ptr() as *mut _,
            length: ELEM_ENCODE_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        // null uri, attr byte local name
        qname.uri = &nullstr_c;
        let attr_byte_str = CString::new(ATTR_BYTE_STR).unwrap();
        qname.localName = &StringType {
            str_: attr_byte_str.as_ptr() as *mut _,
            length: ATTR_BYTE_STR.len(),
        };
        let ec = (serialize.attribute).unwrap()(
            &mut testStrm as *mut _,
            qname,
            1,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let mut chVal = nullstr_c.clone();
        let fiftyfive = CString::new("55").unwrap();
        let ec = asciiToString(
            fiftyfive.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [53, 53],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let attr_version_str = CString::new(ATTR_VERSION_STR).unwrap();
        qname.localName = &StringType {
            str_: attr_version_str.as_ptr() as *mut _,
            length: ATTR_VERSION_STR.len(),
        };
        let ec = (serialize.attribute).unwrap()(
            &mut testStrm as *mut _,
            qname,
            1,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let pointtwo = CString::new("0.2").unwrap();
        let ec = asciiToString(
            pointtwo.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [0x30, 0x2e, 0x32],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let example =
            CString::new("This is an example of serializing EXI streams using EXIP low level API")
                .unwrap();
        let ec = asciiToString(
            example.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [
                0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x6e, 0x20, 0x65, 0x78, 0x61,
                0x6d, 0x70, 0x6c, 0x65, 0x20, 0x6f, 0x66, 0x20, 0x73, 0x65, 0x72, 0x69, 0x61, 0x6c,
                0x69, 0x7a, 0x69, 0x6e, 0x67, 0x20, 0x45, 0x58, 0x49, 0x20, 0x73, 0x74, 0x72, 0x65,
                0x61, 0x6d, 0x73, 0x20, 0x75, 0x73, 0x69, 0x6e, 0x67, 0x20, 0x45, 0x58, 0x49, 0x50,
                0x20, 0x6c, 0x6f, 0x77, 0x20, 0x6c, 0x65, 0x76, 0x65, 0x6c, 0x20, 0x41, 0x50, 0x49
            ],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _);
        assert_eq!(ec, 0);

        qname.uri = &ns_str_c;
        let elem_descr_str = CString::new(ELEM_DESCR_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_descr_str.as_ptr() as *mut _,
            length: ELEM_DESCR_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let test = CString::new("This is a test of processing XML schemes with multiple XSD files")
            .unwrap();
        let ec = asciiToString(
            test.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [
                0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x20, 0x74, 0x65, 0x73, 0x74,
                0x20, 0x6f, 0x66, 0x20, 0x70, 0x72, 0x6f, 0x63, 0x65, 0x73, 0x73, 0x69, 0x6e, 0x67,
                0x20, 0x58, 0x4d, 0x4c, 0x20, 0x73, 0x63, 0x68, 0x65, 0x6d, 0x65, 0x73, 0x20, 0x77,
                0x69, 0x74, 0x68, 0x20, 0x6d, 0x75, 0x6c, 0x74, 0x69, 0x70, 0x6c, 0x65, 0x20, 0x58,
                0x53, 0x44, 0x20, 0x66, 0x69, 0x6c, 0x65, 0x73
            ],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _);
        assert_eq!(ec, 0);

        let ns_nested_str = CString::new(NS_NESTED_STR).unwrap();
        qname.uri = &StringType {
            str_: ns_nested_str.as_ptr() as *mut _,
            length: NS_NESTED_STR.len(),
        };
        let elem_test_setup_str = CString::new(ELEM_TEST_SETUP_STR).unwrap();

        qname.localName = &StringType {
            str_: elem_test_setup_str.as_ptr() as *mut _,
            length: ELEM_TEST_SETUP_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);

        let attr_goal_str = CString::new(ATTR_GOAL_STR).unwrap();
        qname.uri = &nullstr_c;
        qname.localName = &StringType {
            str_: attr_goal_str.as_ptr() as *mut _,
            length: ATTR_GOAL_STR.len(),
        };

        let ec = (serialize.attribute).unwrap()(
            &mut testStrm as *mut _,
            qname,
            1,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let verify = CString::new("Verify that the implementation works!").unwrap();
        let ec = asciiToString(
            verify.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [
                0x56, 0x65, 0x72, 0x69, 0x66, 0x79, 0x20, 0x74, 0x68, 0x61, 0x74, 0x20, 0x74, 0x68,
                0x65, 0x20, 0x69, 0x6d, 0x70, 0x6c, 0x65, 0x6d, 0x65, 0x6e, 0x74, 0x61, 0x74, 0x69,
                0x6f, 0x6e, 0x20, 0x77, 0x6f, 0x72, 0x6b, 0x73, 0x21
            ],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let simple = CString::new("Simple test element with single attribute").unwrap();
        let ec = asciiToString(
            simple.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [
                0x53, 0x69, 0x6d, 0x70, 0x6c, 0x65, 0x20, 0x74, 0x65, 0x73, 0x74, 0x20, 0x65, 0x6c,
                0x65, 0x6d, 0x65, 0x6e, 0x74, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x73, 0x69, 0x6e,
                0x67, 0x6c, 0x65, 0x20, 0x61, 0x74, 0x74, 0x72, 0x69, 0x62, 0x75, 0x74, 0x65
            ],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </testSetup>
        assert_eq!(ec, 0);

        qname.uri = &ns_str_c;
        let elem_type_test_str = CString::new(ELEM_TYPE_TEST_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_type_test_str.as_ptr() as *mut _,
            length: ELEM_TYPE_TEST_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let attr_id_str = CString::new(ATTR_ID_STR).unwrap();
        qname.uri = &nullstr_c;
        qname.localName = &StringType {
            str_: attr_id_str.as_ptr() as *mut _,
            length: ATTR_ID_STR.len(),
        };

        let ec = (serialize.attribute).unwrap()(
            &mut testStrm as *mut _,
            qname,
            1,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let id = CString::new("1001").unwrap();
        let ec = asciiToString(
            id.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [0x31, 0x30, 0x30, 0x31],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        qname.uri = &StringType {
            str_: ns_nested_str.as_ptr() as *mut _,
            length: NS_NESTED_STR.len(),
        };
        let elem_bool_str = CString::new(ELEM_BOOL_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_bool_str.as_ptr() as *mut _,
            length: ELEM_BOOL_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let t = CString::new("true").unwrap();
        let ec = asciiToString(
            t.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [0x74, 0x72, 0x75, 0x65],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </bool>
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </type-test>
        assert_eq!(ec, 0);

        qname.uri = &ns_str_c;
        let elem_ext_types_str = CString::new(ELEM_EXT_TYPES_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_ext_types_str.as_ptr() as *mut _,
            length: ELEM_EXT_TYPES_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        qname.uri = &nullstr_c;
        let elem_byte_types_str = CString::new(ELEM_BYTE_TYPES_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_byte_types_str.as_ptr() as *mut _,
            length: ELEM_BYTE_TYPES_STR.len(),
        };

        // <byteTest>
        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let eleven = CString::new("11").unwrap();
        let ec = asciiToString(
            eleven.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [0x31, 0x31],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </byteTest>
        assert_eq!(ec, 0);

        qname.uri = &nullstr_c;
        let elem_date_types_str = CString::new(ELEM_DATE_TYPES_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_date_types_str.as_ptr() as *mut _,
            length: ELEM_DATE_TYPES_STR.len(),
        };

        // <dateTimeTest>
        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let time = CString::new("2012 Jul 31 13:33").unwrap();
        let ec = asciiToString(
            time.as_ptr(),
            &mut chVal as *mut _,
            &mut testStrm.memList as *mut _,
            0,
        );
        assert_eq!(ec, 0);
        assert_eq!(
            [
                0x32, 0x30, 0x31, 0x32, 0x20, 0x4a, 0x75, 0x6c, 0x20, 0x33, 0x31, 0x20, 0x31, 0x33,
                0x3a, 0x33, 0x33
            ],
            std::slice::from_raw_parts(chVal.str_, chVal.length)
        );

        let ec = (serialize.stringData).unwrap()(&mut testStrm as *mut _, chVal);
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </dateTimeTest>
        assert_eq!(ec, 0);

        qname.uri = &nullstr_c;
        let elem_bin_types_str = CString::new(ELEM_BIN_TYPES_STR).unwrap();
        qname.localName = &StringType {
            str_: elem_bin_types_str.as_ptr() as *mut _,
            length: ELEM_BIN_TYPES_STR.len(),
        };

        // <binaryTest>
        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let b64_data = CString::new(SOME_BINARY_DATA_BASE64).unwrap();
        let ec = (serialize.stringData).unwrap()(
            &mut testStrm as *mut _,
            StringType {
                str_: b64_data.as_ptr() as *mut _,
                length: SOME_BINARY_DATA_BASE64.len(),
            },
        );
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </binaryTest>
        assert_eq!(ec, 0);

        let elem_enum_types_str = CString::new(ELEM_ENUM_TYPES_STR).unwrap();

        qname.uri = &nullstr_c;
        qname.localName = &StringType {
            str_: elem_enum_types_str.as_ptr() as *mut _,
            length: ELEM_ENUM_TYPES_STR.len(),
        };

        let ec = (serialize.startElement).unwrap()(
            &mut testStrm as *mut _,
            qname,
            &mut valueType as *mut _,
        );
        assert_eq!(ec, 0);
        assert_eq!(valueType, 0);

        let enum_data = CString::new(ENUM_DATA_4).unwrap();
        let ec = (serialize.stringData).unwrap()(
            &mut testStrm as *mut _,
            StringType {
                str_: enum_data.as_ptr() as *mut _,
                length: ENUM_DATA_4.len(),
            },
        );
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </enumTest>
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </extendedTypeTest>
        assert_eq!(ec, 0);

        let ec = (serialize.endElement).unwrap()(&mut testStrm as *mut _); // </MultipleXSDsTest>
        assert_eq!(ec, 0);

        let ec = (serialize.endDocument).unwrap()(&mut testStrm as *mut _);
        assert_eq!(ec, 0);

        assert_eq!(
            [
                36, 69, 88, 73, 160, 2, 172, 2, 12, 178, 18, 52, 58, 58, 56, 29, 23, 151, 187, 187,
                187, 151, 54, 58, 58, 151, 57, 178, 151, 162, 164, 169, 166, 32, 161, 23, 185, 177,
                180, 50, 182, 176, 150, 186, 50, 185, 186, 8, 166, 186, 182, 58, 52, 184, 54, 50,
                172, 41, 162, 57, 170, 50, 185, 186, 80, 49, 21, 97, 37, 65, 21, 185, 141, 189,
                145, 149, 201, 33, 46, 140, 174, 110, 136, 79, 46, 140, 160, 134, 166, 180, 132,
                59, 50, 185, 57, 180, 183, 183, 2, 152, 23, 25, 90, 66, 163, 67, 75, 153, 3, 75,
                153, 3, 11, 113, 3, 43, 195, 11, 107, 131, 99, 41, 3, 123, 49, 3, 155, 43, 147, 75,
                11, 99, 75, 211, 75, 115, 57, 2, 42, 194, 73, 3, 155, 163, 147, 43, 11, 107, 153,
                3, 171, 155, 75, 115, 57, 2, 42, 194, 74, 129, 3, 99, 123, 185, 3, 99, 43, 179, 43,
                97, 2, 10, 130, 74, 129, 140, 140, 174, 108, 110, 77, 46, 14, 141, 45, 237, 218,
                18, 163, 67, 75, 153, 3, 75, 153, 3, 9, 3, 163, 43, 155, 161, 3, 123, 49, 3, 131,
                147, 123, 27, 43, 155, 155, 75, 115, 57, 2, 194, 106, 97, 3, 155, 27, 67, 43, 107,
                43, 153, 3, 187, 75, 163, 65, 3, 107, 171, 99, 163, 75, 131, 99, 41, 2, 194, 154,
                33, 3, 51, 75, 99, 43, 154, 2, 54, 135, 71, 71, 3, 162, 242, 247, 119, 119, 114,
                230, 199, 71, 82, 231, 54, 82, 244, 84, 149, 52, 196, 20, 34, 246, 230, 87, 55, 70,
                86, 66, 215, 135, 54, 64, 167, 70, 87, 55, 69, 54, 87, 71, 87, 4, 130, 179, 183,
                176, 182, 19, 171, 50, 185, 52, 179, 60, 144, 58, 52, 48, 186, 16, 58, 52, 50, 144,
                52, 182, 184, 54, 50, 182, 178, 183, 58, 48, 186, 52, 183, 183, 16, 59, 183, 185,
                53, 185, 144, 242, 181, 54, 150, 215, 6, 198, 82, 7, 70, 87, 55, 66, 6, 86, 198,
                86, 214, 86, 231, 66, 7, 118, 151, 70, 130, 7, 54, 150, 230, 118, 198, 82, 6, 23,
                71, 71, 38, 150, 39, 87, 70, 86, 129, 78, 143, 46, 12, 165, 174, 140, 174, 110,
                137, 3, 105, 100, 6, 49, 48, 48, 49, 212, 21, 137, 189, 189, 179, 6, 116, 114, 117,
                101, 34, 8, 178, 188, 58, 50, 183, 50, 50, 178, 42, 60, 184, 50, 170, 50, 185, 186,
                68, 37, 137, 229, 209, 149, 81, 149, 205, 211, 4, 49, 49, 68, 53, 145, 133, 209,
                149, 81, 165, 181, 149, 81, 149, 205, 211, 19, 50, 48, 49, 50, 32, 74, 117, 108,
                32, 51, 49, 32, 49, 51, 58, 51, 51, 66, 22, 196, 210, 220, 194, 228, 242, 168, 202,
                230, 233, 135, 52, 153, 185, 178, 27, 179, 48, 186, 61, 60, 48, 178, 49, 9, 101,
                110, 117, 109, 84, 101, 115, 116, 193, 90, 25, 90, 142, 0
            ],
            std::slice::from_raw_parts(testStrm.buffer.buf as *mut u8, testStrm.buffer.bufLen)
        )
    }
}
