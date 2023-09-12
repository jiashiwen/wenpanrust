use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/sample.proto")?;
    tonic_build::compile_protos("proto/route_guide.proto")?;
    // tonic_build::compile_protos("proto/echo.proto")?;

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("stream_descriptor.bin"))
        .compile(&["proto/echo.proto"], &["proto"])
        .unwrap();

    Ok(())
}
