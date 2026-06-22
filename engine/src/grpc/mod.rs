//! gRPC HTTP/2 service implementations (tonic)
//! Phase 4: Native gRPC + gRPC-Web proxy
//! 
//! All business logic is shared with Connect-RPC handlers in connect_rpc/

pub mod generated;
pub mod query;
pub mod schema;
pub mod share;
pub mod export;
pub mod web_proxy;

use tonic::transport::Server;
use tracing::info;

use crate::AppState;

/// Start gRPC server on dedicated port (default 3002)
/// gRPC-Web is served on the same port with a proxy layer
pub async fn start_grpc_server(
    state: AppState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port).parse()?;
    
    info!("Starting gRPC server on {}", addr);
    
        // Build reflection service for grpcurl/discovery
        let reflection_service = if generated::FILE_DESCRIPTOR_SET.is_empty() {
            // Stub mode — no reflection available
            tracing::warn!("gRPC reflection unavailable — proto files not generated yet");
            None
        } else {
            Some(
                tonic_reflection::server::Builder::configure()
                    .register_encoded_file_descriptor_set(generated::FILE_DESCRIPTOR_SET)
                    .build()?
            )
        };
    
    let share_service = share::ShareGrpcService::new(state.clone());
    let query_service = query::QueryGrpcService::new(state.clone());
    let schema_service = schema::SchemaGrpcService::new(state.clone());
    let export_service = export::ExportGrpcService::new(state.clone());
    
    // gRPC-Web proxy layer
    let web_proxy = web_proxy::grpc_web_proxy();
    
    let mut server = Server::builder()
        .accept_http1(true) // Required for gRPC-Web
        .layer(web_proxy);
    
    // Add reflection if available
    if let Some(reflection) = reflection_service {
        server = server.add_service(reflection);
    }
    
    server
        .add_service(generated::share_service_server::ShareServiceServer::new(share_service))
        .add_service(generated::query_service_server::QueryServiceServer::new(query_service))
        .add_service(generated::schema_service_server::SchemaServiceServer::new(schema_service))
        .add_service(generated::export_service_server::ExportServiceServer::new(export_service))
        .serve(addr)
        .await?;
    
    Ok(())
}

/// gRPC status codes for error mapping
pub fn map_error_to_status(e: &str) -> tonic::Status {
    if e.contains("not found") || e.contains("not_found") {
        tonic::Status::not_found(e)
    } else if e.contains("permission") || e.contains("unauthorized") || e.contains("unauthenticated") {
        tonic::Status::permission_denied(e)
    } else if e.contains("invalid") || e.contains("bad request") {
        tonic::Status::invalid_argument(e)
    } else if e.contains("timeout") || e.contains("deadline") {
        tonic::Status::deadline_exceeded(e)
    } else {
        tonic::Status::internal(e)
    }
}

/// Convert JSON Value to protobuf Value
pub fn json_to_prost_value(v: &serde_json::Value) -> prost_types::Value {
    use prost_types::value::Kind;
    
    match v {
        serde_json::Value::Null => prost_types::Value { kind: Some(Kind::NullValue(0)) },
        serde_json::Value::Bool(b) => prost_types::Value { kind: Some(Kind::BoolValue(*b)) },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                prost_types::Value { kind: Some(Kind::NumberValue(i as f64)) }
            } else if let Some(f) = n.as_f64() {
                prost_types::Value { kind: Some(Kind::NumberValue(f)) }
            } else {
                prost_types::Value { kind: Some(Kind::StringValue(n.to_string())) }
            }
        }
        serde_json::Value::String(s) => prost_types::Value { kind: Some(Kind::StringValue(s.clone())) },
        serde_json::Value::Array(arr) => {
            let values: Vec<prost_types::Value> = arr.iter().map(json_to_prost_value).collect();
            prost_types::Value { kind: Some(Kind::ListValue(prost_types::ListValue { values })) }
        }
        serde_json::Value::Object(obj) => {
            let fields: std::collections::HashMap<String, prost_types::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect();
            prost_types::Value { kind: Some(Kind::StructValue(prost_types::Struct { fields })) }
        }
    }
}

/// Convert prost Value to JSON Value
pub fn prost_value_to_json(v: &prost_types::Value) -> serde_json::Value {
    use prost_types::value::Kind;
    
    match &v.kind {
        Some(Kind::NullValue(_)) => serde_json::Value::Null,
        Some(Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(Kind::NumberValue(n)) => {
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)))
            }
        }
        Some(Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(Kind::ListValue(l)) => {
            serde_json::Value::Array(l.values.iter().map(prost_value_to_json).collect())
        }
        Some(Kind::StructValue(s)) => {
            serde_json::Value::Object(s.fields.iter().map(|(k, v)| (k.clone(), prost_value_to_json(v))).collect())
        }
        None => serde_json::Value::Null,
    }
}
