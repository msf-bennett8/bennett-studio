//! gRPC-Web proxy layer
//! Converts gRPC-Web requests to native gRPC and vice versa
//! Allows browser clients to call gRPC services over HTTP/1.1

use tonic_web::GrpcWebLayer;
use tower::ServiceBuilder;

/// Create gRPC-Web proxy middleware
/// This layer intercepts gRPC-Web requests and converts them to standard gRPC
pub fn grpc_web_proxy() -> GrpcWebLayer {
    GrpcWebLayer::new()
}

/// Full service builder with gRPC-Web, CORS, and compression
pub fn grpc_service_stack() -> ServiceBuilder<tower::ServiceBuilder> {
    ServiceBuilder::new()
        .layer(GrpcWebLayer::new())
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::compression::CompressionLayer::new())
}
