use cosmwasm_std::{
    Addr, CosmosMsg,
    DepsMut, Env, MessageInfo, Response,
};
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::state::get_decimals;
use crate::helpers; 
use crate::{
    state::{
         MANAGER_CONFIG,
        PAIRS,
    },
    ContractError,
};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use cosmwasm_std::coins;
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale::pool_network::asset::is_factory_token;
#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::MsgCreateDenom;
use white_whale::pool_network::querier::query_balance;

#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

use cosmwasm_std::{Decimal, Uint128};

// Stuff like Swap, Swap through router and any other stuff related to swapping
pub fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Asset,
    ask_asset: AssetInfo,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the deposit feature is enabled
    if !config.feature_toggle.deposits_enabled {
        return Err(ContractError::OperationDisabled("swap".to_string()));
    }

    offer_asset.assert_sent_native_token_balance(&info)?;

    let asset_infos = [ask_asset.clone(), offer_asset.info.clone()];
    let ask_asset = Asset {
        info: ask_asset,
        amount: Uint128::zero(),
    };
    let assets = [ask_asset.clone(), offer_asset.clone()];
    // Load assets, pools and pair info
    let (_assets_vec, pools, pair_info) = match assets {
        // For TWO assets we use the constant product logic
        assets if assets.len() == 2 => {
            let pair_info = PAIRS.load(deps.storage, pair_identifier)?;
            println!("After load");
            println!("{:?}", pair_info);
            let pools: [Asset; 2] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_balance(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_balance(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        // For both THREE and N we use the same logic; stableswap or eventually conc liquidity
        assets if assets.len() == 3 => {
            let pair_info = PAIRS.load(deps.storage, pair_identifier)?;

            // TODO: this is fucked, rework later after constant product working
            let asset_infos = [
                offer_asset.info.clone(),
                ask_asset.info.clone(),
                ask_asset.info.clone(),
            ];
            let assets = [offer_asset.clone(), ask_asset.clone(), ask_asset];

            let pools: [Asset; 3] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_balance(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_balance(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[2].clone(),
                    amount: asset_infos[2].query_balance(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        _ => {
            return Err(ContractError::TooManyAssets {
                assets_provided: assets.len(),
            })
        }
    };
    // determine what's the offer and ask pool based on the offer_asset
    let offer_pool: Asset;
    let ask_pool: Asset;
    let offer_decimal: u8;
    let ask_decimal: u8;
    let decimals = get_decimals(&pair_info);
    println!("After decimals");
    // We now have the pools and pair info; we can now calculate the swap
    // Verify the pool
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
        offer_decimal = decimals[0];
        ask_decimal = decimals[1];
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();

        offer_decimal = decimals[1];
        ask_decimal = decimals[0];
    } else {
        return Err(ContractError::AssetMismatch {});
    }
    println!("Found pools");
    let _attributes = vec![
        ("action", "swap"),
        ("pair_type", pair_info.pair_type.get_label()),
    ];

    let mut messages: Vec<CosmosMsg> = vec![];

    let receiver = to.unwrap_or_else(|| sender.clone());

    // TODO: Add the swap logic here
    let offer_amount = offer_asset.amount;
    let pool_fees = pair_info.pool_fees;

    let swap_computation = helpers::compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_amount,
        pool_fees,
        &pair_info.pair_type,
        offer_decimal,
        ask_decimal,
    )?;

    let return_asset = Asset {
        info: ask_pool.info.clone(),
        amount: swap_computation.return_amount,
    };

    // Assert spread and other operations
    // check max spread limit if exist
    helpers::assert_max_spread(
        belief_price,
        max_spread,
        offer_asset.clone(),
        return_asset.clone(),
        swap_computation.spread_amount,
        offer_decimal,
        ask_decimal,
    )?;
    println!("After spread");
    println!("Return amount: {}", return_asset.amount);
    // TODO; add the swap messages
    if !swap_computation.return_amount.is_zero() {
        messages.push(return_asset.into_msg(receiver.clone())?);
    }
    println!("After return amount: {:?}", swap_computation);

    // burn ask_asset from the pool
    // if !swap_computation.burn_fee_amount.is_zero() {
    //     let burn_asset = Asset {
    //         info: ask_pool.info.clone(),
    //         amount: swap_computation.burn_fee_amount,
    //     };

    //     store_fee(
    //         deps.storage,
    //         burn_asset.amount,
    //         burn_asset.clone().get_id(),
    //         ALL_TIME_BURNED_FEES,
    //     )?;

    //     messages.push(burn_asset.into_burn_msg()?);
    // }

    // Store the protocol fees generated by this swap. The protocol fees are collected on the ask
    // asset as shown in [compute_swap]
    // store_fee(
    //     deps.storage,
    //     swap_computation.protocol_fee_amount,
    //     ask_pool.clone().get_id(),
    //     COLLECTABLE_PROTOCOL_FEES,
    // )?;
    // store_fee(
    //     deps.storage,
    //     swap_computation.protocol_fee_amount,
    //     ask_pool.clone().get_id(),
    //     TOTAL_COLLECTED_PROTOCOL_FEES,
    // )?;
    println!("After fees");

    // 1. send collateral token from the contract to a user
    // 2. stores the protocol fees
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap"),
        ("sender", sender.as_str()),
        ("receiver", receiver.as_str()),
        ("offer_asset", &offer_asset.info.to_string()),
        ("ask_asset", &ask_pool.info.to_string()),
        ("offer_amount", &offer_amount.to_string()),
        ("return_amount", &swap_computation.return_amount.to_string()),
        ("spread_amount", &swap_computation.spread_amount.to_string()),
        (
            "swap_fee_amount",
            &swap_computation.swap_fee_amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            &swap_computation.protocol_fee_amount.to_string(),
        ),
        (
            "burn_fee_amount",
            &swap_computation.burn_fee_amount.to_string(),
        ),
        ("swap_type", pair_info.pair_type.get_label()),
    ]))
}
