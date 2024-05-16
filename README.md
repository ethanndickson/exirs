# exirs

`exirs` provides Rust bindings for [EXIP](https://sourceforge.net/projects/exip/), an embeddable EXI processor written in C.

This library is currently a WIP, and requires extensive further testing.

# Documentation
todo

# Examples
Writing a simple schemaless document:
```rust
use exirs::{
    config::{Header, Options},
    data::{Event, Name, Value},
    Writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::default().strict(true);
    let header = Header::with_options(options).has_cookie(true);
    let mut builder = Writer::new(header, None)?;
    builder.add(Event::StartDocument)?;
    builder.add(Event::StartElement(Name {
        local_name: "MultipleXSDsTest",
        namespace: "http://www.ltu.se/EISLAB/schema-test",
        prefix: None,
    }))?;
    builder.add(Event::Value(Value::String(
        "This is an example of serializing EXI streams using EXIP low level API",
    )))?;
    builder.add(Event::EndElement)?;
    builder.add(Event::EndDocument)?;
    println!("{:#04X?}", builder.get());
    Ok(())
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