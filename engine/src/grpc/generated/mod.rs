//! Auto-generated protobuf code
//! Run `cargo build` to regenerate from .proto files

// Include generated code
pub mod bennett {
    pub mod v1 {
        include!("share.rs");
        include!("query.rs");
        include!("schema.rs");
        include!("export.rs");
    }
}

// Re-export for convenience
pub use bennett::v1::*;

// File descriptor set for reflection
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("bennett_descriptor");

// Re-export tonic service traits
pub use bennett::v1::share_service_server::{ShareService, ShareServiceServer};
pub use bennett::v1::query_service_server::{QueryService, QueryServiceServer};
pub use bennett::v1::schema_service_server::{SchemaService, SchemaServiceServer};
pub use bennett::v1::export_service_server::{ExportService, ExportServiceServer};
