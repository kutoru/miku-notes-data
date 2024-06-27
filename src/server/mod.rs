use tonic::{transport::{Server, Body}, codegen::http::Request, async_trait, Status};
use tonic_middleware::{RequestInterceptorLayer, RequestInterceptor};

use crate::types::AppState;

mod files;
mod tags;
mod notes;
mod file_server;

pub async fn start_grpc_server(state: &AppState, addr: &str) -> anyhow::Result<()> {
    let interceptor = RequestInterceptorLayer::new(Interceptor {});

    let files_service = files::get_service(state.clone());
    let tags_service = tags::get_service(state.clone());
    let notes_service = notes::get_service(state.clone());

    println!("GRPC server listening on {}", addr);

    Server::builder()
        .layer(interceptor)
        .add_service(files_service)
        .add_service(tags_service)
        .add_service(notes_service)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}

pub async fn start_file_server(state: &AppState, addr: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let app = file_server::get_router(state);

    println!("File server listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Clone)]
pub struct Interceptor {}

// auth validation examples
// https://crates.io/crates/tonic-middleware
// https://github.com/teimuraz/tonic-middleware/blob/main/example/src/server.rs

#[async_trait]
impl RequestInterceptor for Interceptor {
    async fn intercept(
        &self,
        req: Request<Body>
    ) -> Result<Request<Body>, Status> {
        println!("Request: {} -> {}", req.method(), req.uri().path());
        Ok(req)
    }
}
