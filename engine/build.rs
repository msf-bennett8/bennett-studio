use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    let proto_dir = Path::new("../shared/proto/bennett/v1");
    let out_dir = Path::new("src/grpc/generated");

    // Ensure output directory exists
    std::fs::create_dir_all(out_dir)?;

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
        // No proto files yet — create stub module so compilation doesn't fail
        let stub = r#"// Auto-generated stub — proto files not found
// Run `cargo build` from workspace root after creating .proto files

pub mod bennett {
    pub mod v1 {
        // Stub types to allow compilation
        include!("stub.rs");
    }
}

pub use bennett::v1::*;

pub const FILE_DESCRIPTOR_SET: &[u8] = &[];
"#;
        std::fs::write(out_dir.join("mod.rs"), stub)?;

        // Create stub.rs with minimal types
        let stub_rs = r#"// Stub implementations — replace with protoc-generated code
// Generate with: protoc --prost_out=. --tonic_out=. *.proto

// Placeholder types
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateShareRequest {
    #[prost(string, tag = "1")]
    pub database_id: String,
    #[prost(string, tag = "2")]
    pub permission: String,
    #[prost(string, repeated, tag = "3")]
    pub tables: Vec<String>,
    #[prost(string, tag = "4")]
    pub rls: String,
    #[prost(int64, tag = "5")]
    pub duration_hours: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateShareResponse {
    #[prost(string, tag = "1")]
    pub code: String,
    #[prost(string, tag = "2")]
    pub url: String,
    #[prost(string, tag = "3")]
    pub token: String,
    #[prost(string, tag = "4")]
    pub expires_at: String,
}

// Add more stub types as needed...
"#;
        std::fs::write(out_dir.join("stub.rs"), stub_rs)?;

        println!("cargo:warning=No .proto files found. Created stub module.");
        return Ok(());
    }

    // Generate from proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("bennett_descriptor.bin"))
        .out_dir(out_dir)
        .compile_protos(&existing_protos, &[proto_dir.to_str().unwrap()])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Rewrite mod.rs to include generated files
    let mod_rs = r#"//! Auto-generated protobuf code
// Run `cargo build` to regenerate from .proto files

pub mod bennett {
    pub mod v1 {
        include!("bennett.v1.rs");
    }
}

pub use bennett::v1::*;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("bennett_descriptor");
"#;
    std::fs::write(out_dir.join("mod.rs"), mod_rs)?;

    Ok(())
}