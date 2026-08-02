use vibea3::{Kbin, RpcContext, RpcResult, RpcServer, rpc, rpc_app};

#[derive(Clone)]
struct State;

#[derive(Kbin)]
#[kbin(node = "services")]
struct ServicesRequest {}

#[derive(Kbin)]
#[kbin(node = "services")]
struct ServicesResponse {}

#[rpc("services.get")]
async fn services_get(
    _ctx: RpcContext<State>,
    _request: ServicesRequest,
) -> RpcResult<ServicesResponse> {
    Ok(ServicesResponse {})
}

#[tokio::main]
async fn main() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
