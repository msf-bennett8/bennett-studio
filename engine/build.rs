use std::io::Result;

fn main() -> Result<()> {
    // Compile protobuf definitions
    let proto_files = [
        "../shared/proto/bennett/v1/share.proto",
        "../shared/proto/bennett/v1/query.proto",
        "../shared/proto/bennett/v1/schema.proto",
        "../shared/proto/bennett/v1/export.proto",
    ];
    
    for proto in &proto_files {
        println!("cargo:rerun-if-changed={}", proto);
    }
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/grpc/generated")
        .compile(&proto_files, &["../shared/proto"])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    Ok(())
}