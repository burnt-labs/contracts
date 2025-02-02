#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    SerdeJSON(#[from] serde_json::Error),

    #[error("r1cs synthesis error")]
    R1CS(#[from] ark_relations::r1cs::SynthesisError),

    #[error(transparent)]
    ArkSerialization(#[from] ark_serialize::SerializationError),

    #[error("dkim invalid")]
    InvalidDkim,

    #[error(transparent)]
    EncodeError(#[from] cosmos_sdk_proto::prost::EncodeError),

    #[error(transparent)]
    DecodeError(#[from] cosmos_sdk_proto::prost::DecodeError),
}

pub type ContractResult<T> = Result<T, ContractError>;
