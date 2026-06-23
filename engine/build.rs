use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    let proto_dir = Path::new("../shared/proto/bennett/v1");
    let out_dir_str = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    let proto_files = [
        "../shared/proto/bennett/v1/share.proto",
        "../shared/proto/bennett/v1/query.proto",
        "../shared/proto/bennett/v1/schema.proto",
        "../shared/proto/bennett/v1/export.proto",
    ];

    // Check if proto files exist
    let mut existing_protos = Vec::new();
    for proto in &proto_files {
        if Path::new(proto).exists() {
            println!("cargo:rerun-if-changed={}", proto);
            existing_protos.push(proto.to_string());
        }
    }

    if existing_protos.is_empty() {
        println!("cargo:warning=No .proto files found. Skipping proto generation.");
        return Ok(());
    }

    // Generate from proto files into OUT_DIR
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("bennett_descriptor.bin"))
        .out_dir(out_dir)
        .compile_protos(&existing_protos, &[proto_dir.to_str().unwrap()])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(())
}
