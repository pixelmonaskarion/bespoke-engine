use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("resources.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    write!(
        &mut file,
        "static RESOURCES: phf::Map<&'static str, &'static [u8]> = {}",
        phf_codegen::Map::<&[u8]>::new()
            .build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();
}