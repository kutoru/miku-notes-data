use tonic::transport::Server;
use tonic_middleware::RequestInterceptorLayer;

use crate::types::{AppState, Interceptor};

mod files;
mod tags;
mod notes;
mod shelves;

pub async fn start(state: &AppState, port: u16, service_token: &str) -> anyhow::Result<()> {
    let interceptor = RequestInterceptorLayer::new(Interceptor {
        auth_value: format!("Bearer {}", service_token),
    });

    let files_service = files::get_service(state.clone());
    let tags_service = tags::get_service(state.clone());
    let notes_service = notes::get_service(state.clone());
    let shelves_service = shelves::get_service(state.clone());

    let addr = format!("127.0.0.1:{port}");
    println!("Data service listening on {addr}");

    Server::builder()
        .layer(interceptor)
        .add_service(files_service)
        .add_service(tags_service)
        .add_service(notes_service)
        .add_service(shelves_service)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
