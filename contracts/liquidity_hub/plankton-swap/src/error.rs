use cosmwasm_std::StdError;
use thiserror::Error;
use crate::commands::MAX_ASSETS_PER_POOL;
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("The provided assets are both the same")]
    SameAsset {},

    #[error("More assets provided than is supported the max is currently {MAX_ASSETS_PER_POOL}, you provided {assets_provided}")]
    TooManyAssets { assets_provided: usize},

    #[error("{asset} is invalid")]
    InvalidAsset { asset: String },

    #[error("Pair already exist")]
    ExistingPair {},
}
