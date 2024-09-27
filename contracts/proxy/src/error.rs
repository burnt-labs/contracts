#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Encode(#[from] cosmos_sdk_proto::prost::EncodeError),

    #[error(transparent)]
    Decode(#[from] cosmos_sdk_proto::prost::DecodeError),
    
    #[error("unauthorized")]
    Unauthorized,

    #[error("invalid_code_id")]
    InvalidCodeID{contract: String, code_id: u64},
    
    #[error("invalid_msg_type")]
    InvalidMsgType
}

pub type ContractResult<T> = Result<T, ContractError>;