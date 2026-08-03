mod basic;
mod profile;
mod save;

pub(super) fn database_error(error: String) -> vibea3::RpcError {
    vibea3::RpcError::new(1, format!("database_error:{error}"))
}

pub(super) fn invalid_session() -> vibea3::RpcError {
    vibea3::RpcError::protocol_status(1)
}
