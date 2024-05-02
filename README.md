# exirs

`exirs` provides Rust bindings for [EXIP](https://sourceforge.net/projects/exip/), an embeddable EXI processor written in C.

The library is currently a WIP, and only supports reading and writing schemaless documents.


# Examples
Writing a simple schemaless document:
```rust
use exirs::config::{EXIHeader, EXIOptionFlags};
use exirs::data::Name;
use exirs::events::SchemalessEvent;
use exirs::writer::SchemalessBuilder;

fn main() {
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
    println!("{:?}",builder.get());
}
```


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

`exirs` distributes code from EXIP, under the license contained in [`THIRD_PARTY`](THIRD_PARTY).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.