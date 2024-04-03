use std::fmt;

use crate::pool_network::{
    asset::PairType,
    factory::NativeTokenDecimalsResponse,
    pair::{PoolFee, ReverseSimulationResponse, SimulationResponse},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub enum SwapOperation {
    WhaleSwap {
        token_in_denom: String,
        token_out_denom: String,
        pool_identifier: String,
    },
}

impl SwapOperation {
    /// Retrieves the `token_in_denom` used for this swap operation.
    pub fn get_input_asset_info(&self) -> &String {
        match self {
            SwapOperation::WhaleSwap { token_in_denom, .. } => token_in_denom,
        }
    }

    pub fn get_target_asset_info(&self) -> String {
        match self {
            SwapOperation::WhaleSwap {
                token_out_denom, ..
            } => token_out_denom.clone(),
        }
    }
}

impl fmt::Display for SwapOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapOperation::WhaleSwap {
                token_in_denom,
                token_out_denom,
                pool_identifier,
            } => write!(
                f,
                "WhaleSwap {{ token_in_info: {token_in_denom}, token_out_info: {token_out_denom}, pool_identifier: {pool_identifier} }}"
            ),

        }
    }
}

#[cw_serde]
pub struct SwapRoute {
    pub offer_asset_denom: String,
    pub ask_asset_denom: String,
    pub swap_operations: Vec<SwapOperation>,
}

// Used for all swap routes
#[cw_serde]
pub struct SwapRouteResponse {
    pub offer_asset_denom: String,
    pub ask_asset_denom: String,
    pub swap_route: Vec<SwapOperation>,
}

impl fmt::Display for SwapRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SwapRoute {{ offer_asset_info: {}, ask_asset_info: {}, swap_operations: {:?} }}",
            self.offer_asset_denom, self.ask_asset_denom, self.swap_operations
        )
    }
}

// Define a structure for Fees which names a number of defined fee collection types, maybe leaving room for a custom room a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum Fee {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}
#[cw_serde]

pub struct StableSwapParams {
    pub initial_amp: String,
    pub future_amp: String,
    pub initial_amp_block: String,
    pub future_amp_block: String,
}

// Store PairInfo to N
// We define a custom struct for which allows for dynamic but defined pairs
#[cw_serde]
pub struct NPairInfo {
    pub asset_denoms: Vec<String>,
    pub lp_denom: String,
    pub asset_decimals: Vec<u8>,
    // TODO: balances is included in assets, might be redundant
    pub balances: Vec<Uint128>,
    pub assets: Vec<Coin>,
    pub pair_type: PairType,
    pub pool_fees: PoolFee,
    // TODO: Add stable swap params
    // pub stable_swap_params: Option<StableSwapParams>
}
impl NPairInfo {}

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector_addr: String,
    pub owner: String,
    pub pool_creation_fee: Coin,
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        asset_denoms: Vec<String>,
        // TODO: Remap to NPoolFee maybe
        pool_fees: PoolFee,
        pair_type: PairType,
        pair_identifier: Option<String>,
    },
    /// Provides liquidity to the pool
    ProvideLiquidity {
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        pair_identifier: String,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Coin,
        ask_asset_denom: String,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
    },
    // /// Withdraws liquidity from the pool.
    WithdrawLiquidity {
        pair_identifier: String,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals {
        denom: String,
        decimals: u8,
    },

    /// Execute multiple [`SwapOperations`] to allow for multi-hop swaps.
    ExecuteSwapOperations {
        /// The operations that should be performed in sequence.
        ///
        /// The amount in each swap will be the output from the previous swap.
        ///
        /// The first swap will use whatever funds are sent in the [`MessageInfo`].
        operations: Vec<SwapOperation>,
        /// The minimum amount of the output (i.e., final swap operation token) required for the message to succeed.
        minimum_receive: Option<Uint128>,
        /// The (optional) recipient of the output tokens.
        ///
        /// If left unspecified, tokens will be sent to the sender of the message.
        to: Option<String>,
        /// The (optional) maximum spread to incur when performing any swap.
        ///
        /// If left unspecified, there is no limit to what spread the transaction can incur.
        max_spread: Option<Decimal>,
    },
    // /// Swap the offer to ask token. This message can only be called internally by the router contract.
    // ExecuteSwapOperation {
    //     operation: SwapOperation,
    //     to: Option<String>,
    //     max_spread: Option<Decimal>,
    // },
    // /// Checks if the swap amount exceeds the minimum_receive. This message can only be called
    // /// internally by the router contract.
    // AssertMinimumReceive {
    //     asset_info: AssetInfo,
    //     prev_balance: Uint128,
    //     minimum_receive: Uint128,
    //     receiver: String,
    // },
    /// Adds swap routes to the router.
    AddSwapRoutes {
        swap_routes: Vec<SwapRoute>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the decimals for the given native or ibc denom.
    #[returns(NativeTokenDecimalsResponse)]
    NativeTokenDecimals { denom: String },

    /// Simulates a swap.
    #[returns(SimulationResponse)]
    Simulation {
        offer_asset: Coin,
        ask_asset: Coin,
        pair_identifier: String,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        ask_asset: Coin,
        offer_asset: Coin,
        pair_identifier: String,
    },

    /// Gets the swap route for the given offer and ask assets.
    #[returns(Vec<SwapOperation>)]
    SwapRoute {
        offer_asset_denom: String,
        ask_asset_denom: String,
    },
    /// Gets all swap routes registered
    #[returns(Vec<SwapRouteResponse>)]
    SwapRoutes {},

    // /// Simulates swap operations.
    // #[returns(SimulateSwapOperationsResponse)]
    // SimulateSwapOperations {
    //     offer_amount: Uint128,
    //     operations: Vec<SwapOperation>,
    // },
    // /// Simulates a reverse swap operations, i.e. given the ask asset, how much of the offer asset
    // /// is needed to perform the swap.
    // #[returns(SimulateSwapOperationsResponse)]
    // ReverseSimulateSwapOperations {
    //     ask_amount: Uint128,
    //     operations: Vec<SwapOperation>,
    // },
    #[returns(NPairInfo)]
    Pair { pair_identifier: String },
}
