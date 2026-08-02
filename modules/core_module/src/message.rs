use vibea3::{Kbin, RpcContext, RpcError, RpcResult, rpc};

use crate::{State, fault};

#[derive(Kbin)]
#[kbin(node = "message")]
struct Request {}

#[derive(Kbin)]
#[kbin(node = "message")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    #[kbin(repeated)]
    item: Vec<Item>,
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct Item {
    #[kbin(attr)]
    end: i32,
    #[kbin(attr)]
    name: String,
    #[kbin(attr)]
    start: i32,
}

#[rpc("message.get")]
async fn get(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    let machine = ctx
        .state
        .database
        .machine(ctx.srcid.clone().unwrap_or_default())
        .await
        .map_err(database_error)?;
    let maintenance =
        ctx.state.config.maintenance || machine.as_ref().is_some_and(|machine| machine.maintenance);
    let item = if maintenance {
        ["sys.mainte", "sys.eacoin.mainte"]
            .into_iter()
            .map(|name| Item {
                end: 86_400,
                name: name.into(),
                start: 0,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(Response {
        expire: 600,
        fault: fault(&ctx.model),
        item,
    })
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}
