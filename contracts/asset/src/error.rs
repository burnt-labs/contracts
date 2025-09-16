#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContractError {
    // Generic errors
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid input: {msg}")]
    InvalidInput { msg: String },

    #[error("Not found: {msg}")]
    NotFound { msg: String },

    // Specific errors
    #[error("Extension error: {msg}")]
    ExtensionError { msg: String },

    #[error("Plugin error: {msg}")]
    PluginError { msg: String },
    
}

pub type ContractResult<T> = Result<T, ContractError>;