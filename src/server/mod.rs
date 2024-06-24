use tonic::{transport::{Server, Body}, codegen::http::Request, async_trait, Status};
use tonic_middleware::{RequestInterceptorLayer, RequestInterceptor};

use crate::types::AppState;

mod files;
mod tags;
mod notes;

pub async fn start(state: AppState, addr: &str) -> anyhow::Result<()> {
    let interceptor = RequestInterceptorLayer::new(Interceptor {});

    let files_service = files::get_service(state.clone());
    let tags_service = tags::get_service(state.clone());
    let notes_service = notes::get_service(state);

    Server::builder()
        .layer(interceptor)
        .add_service(files_service)
        .add_service(tags_service)
        .add_service(notes_service)
        .serve(addr.parse()?)
        .await?;

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
