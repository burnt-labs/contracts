#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    SerdeJSON(#[from] serde_json::Error),

    #[error("{0}")]
    R1CS(String),

    #[error("{0}")]
    ArkSerialization(String),

    #[error("dkim invalid")]
    InvalidDkim,

    #[error(transparent)]
    EncodeError(#[from] cosmos_sdk_proto::prost::EncodeError),

    #[error(transparent)]
    DecodeError(#[from] cosmos_sdk_proto::prost::DecodeError),
}

pub type ContractResult<T> = Result<T, ContractError>;

impl From<ark_serialize::SerializationError> for ContractError {
    fn from(value: ark_serialize::SerializationError) -> Self {
        Self::ArkSerialization(format!("{:?}", value))
    }
}

impl From<ark_relations::r1cs::SynthesisError> for ContractError {
    fn from(value: ark_relations::r1cs::SynthesisError) -> Self {
        Self::R1CS(format!("{:?}", value))
    }
}