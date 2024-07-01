use tonic::transport::Server;
use tonic_middleware::RequestInterceptorLayer;

use crate::types::{AppState, Interceptor};

mod files;
mod tags;
mod notes;

pub async fn start(state: &AppState, addr: &str, service_token: &str) -> anyhow::Result<()> {
    let interceptor = RequestInterceptorLayer::new(Interceptor {
        auth_value: format!("Bearer {}", service_token),
    });

    let files_service = files::get_service(state.clone());
    let tags_service = tags::get_service(state.clone());
    let notes_service = notes::get_service(state.clone());

    println!("Data server listening on {}", addr);

    Server::builder()
        .layer(interceptor)
        .add_service(files_service)
        .add_service(tags_service)
        .add_service(notes_service)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
