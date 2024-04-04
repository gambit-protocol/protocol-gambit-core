use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response};

use crate::{state::MANAGER_CONFIG, ContractError};

pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

use cosmwasm_std::Decimal;

use super::perform_swap::perform_swap;

#[allow(clippy::too_many_arguments)]
pub fn swap(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Coin,
    _ask_asset: String,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    if !config.feature_toggle.swaps_enabled {
        return Err(ContractError::OperationDisabled("swap".to_string()));
    }

    if cw_utils::one_coin(&info)? != offer_asset {
        return Err(ContractError::AssetMismatch {});
    }

    // perform the swap
    let swap_result = perform_swap(
        deps,
        offer_asset.clone(),
        pair_identifier,
        belief_price,
        max_spread,
    )?;

    // add messages
    let mut messages: Vec<CosmosMsg> = vec![];
    let receiver = to.unwrap_or_else(|| sender.clone());

    // first we add the swap result
    if !swap_result.return_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.to_string(),
            amount: vec![swap_result.return_asset.clone()],
        }));
    }
    // then we add the fees
    if !swap_result.burn_fee_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Burn {
            amount: vec![swap_result.burn_fee_asset.clone()],
        }));
    }

    if !swap_result.protocol_fee_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.fee_collector_addr.to_string(),
            amount: vec![swap_result.protocol_fee_asset.clone()],
        }));
    }

    if !swap_result.swap_fee_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.fee_collector_addr.to_string(),
            amount: vec![swap_result.swap_fee_asset.clone()],
        }));
    }

    // 1. send collateral token from the contract to a user
    // 2. stores the protocol fees
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap"),
        ("sender", sender.as_str()),
        ("receiver", receiver.as_str()),
        ("offer_denom", &offer_asset.denom),
        ("ask_denom", &swap_result.return_asset.denom),
        ("offer_amount", &offer_asset.amount.to_string()),
        (
            "return_amount",
            &swap_result.return_asset.amount.to_string(),
        ),
        ("spread_amount", &swap_result.spread_amount.to_string()),
        (
            "swap_fee_amount",
            &swap_result.swap_fee_asset.amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            &swap_result.protocol_fee_asset.amount.to_string(),
        ),
        (
            "burn_fee_amount",
            &swap_result.burn_fee_asset.amount.to_string(),
        ),
        ("swap_type", swap_result.pair_info.pair_type.get_label()),
    ]))
}