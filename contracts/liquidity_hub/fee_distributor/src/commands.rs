use cosmwasm_std::{DepsMut, MessageInfo, Response, StdError, Uint128};

use terraswap::asset::{Asset, AssetInfo};

use crate::state::{
    get_claimable_epochs, get_current_epoch, get_expiring_epoch, Epoch, CONFIG, EPOCHS,
    LAST_CLAIMED_EPOCH,
};
use crate::ContractError;

/// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
pub fn create_new_epoch(
    deps: DepsMut,
    info: MessageInfo,
    mut fees: Vec<Asset>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only the fee collector can call this function
    if info.sender != config.fee_collector_addr {
        return Err(ContractError::Unauthorized {});
    }

    // make sure the fees match the funds sent
    let invalid_funds: Vec<Asset> = info
        .funds
        .iter()
        .map(|coin| Asset {
            info: AssetInfo::NativeToken {
                denom: coin.denom.clone(),
            },
            amount: coin.amount,
        })
        .filter(|asset| !fees.contains(asset))
        .collect();
    if !invalid_funds.is_empty() {
        return Err(ContractError::AssetMismatch {});
    }

    // forward fees from previous epoch to the new one
    let current_epoch = get_current_epoch(deps.as_ref())?;
    let expiring_epoch = get_expiring_epoch(deps.as_ref())?;
    let unclaimed_fees = expiring_epoch
        .map(|epoch| epoch.available)
        .unwrap_or(vec![]);

    fees = aggregate_fees(fees, unclaimed_fees);

    let new_epoch = Epoch {
        id: current_epoch
            .id
            .checked_add(1)
            .ok_or(StdError::generic_err("couldn't compute epoch id"))?,
        total: fees.clone(),
        available: fees.clone(),
        claimed: vec![],
    };

    EPOCHS.save(deps.storage, &new_epoch.id.to_be_bytes(), &new_epoch)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_new_epoch".to_string()),
        ("new_epoch", new_epoch.id.to_string()),
        ("fees_to_distribute", format!("{:?}", fees)),
    ]))
}

/// Aggregates assets from two fee vectors, summing up the amounts of assets that are the same.
fn aggregate_fees(fees: Vec<Asset>, other_fees: Vec<Asset>) -> Vec<Asset> {
    let mut aggregated_fees = fees;

    for fee in other_fees {
        let mut found = false;
        for aggregated_fee in &mut aggregated_fees {
            if fee.info == aggregated_fee.info {
                aggregated_fee.amount += fee.amount;
                found = true;
                break;
            }
        }

        if !found {
            aggregated_fees.push(fee);
        }
    }

    aggregated_fees
}

pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Query the fee share of the sender based on the ratio of his weight and the global weight at the current moment
    /*
    let config = CONFIG.load(deps.storage)?;
    let staking_weight = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.staking_contract_addr.to_string(),
        msg: to_binary(&())?,
    }))?;

    let fee_share = staking_weight.user_weight / staking_weight.global_weight;*/

    let fee_share = Uint128::from(3u128);

    let mut claimable_epochs = get_claimable_epochs(deps.as_ref())?;
    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &info.sender)?;

    // filter out epochs that have already been claimed by the user
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        claimable_epochs = claimable_epochs
            .into_iter()
            .filter(|epoch| epoch.id > last_claimed_epoch)
            .collect();

        // the user has already claimed fees on all claimable epochs
        if claimable_epochs.is_empty() {
            return Err(ContractError::NothingToClaim {});
        }
    };

    let mut claimable_fees = vec![];
    for mut epoch in claimable_epochs.clone() {
        for fee in epoch.total.iter() {
            let reward = fee.amount.checked_div(fee_share)?;

            // make sure the reward is sound
            let fee_available = epoch
                .available
                .iter()
                .find_map(|available_fee| {
                    if available_fee.info == fee.info {
                        Some(available_fee.amount)
                    } else {
                        None
                    }
                })
                .ok_or(StdError::generic_err("Invalid fee"))?;

            if reward > fee_available {
                return Err(ContractError::InvalidReward {});
            }

            // add the reward to the claimable fees
            claimable_fees = aggregate_fees(
                claimable_fees,
                vec![Asset {
                    info: fee.info.clone(),
                    amount: reward,
                }],
            );

            // modify the epoch to reflect the new available and claimed amount
            epoch.available.iter_mut().for_each(|available_fee| {
                if available_fee.info == fee.info {
                    available_fee.amount -= reward;
                }
            });

            epoch.claimed.iter_mut().for_each(|claimed_fee| {
                if claimed_fee.info == fee.info {
                    claimed_fee.amount += reward;
                }
            });

            EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), &epoch)?;
        }
    }

    // update the last claimed epoch for the user
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &claimable_epochs[0].id)?;

    // send funds to the user
    let mut messages = vec![];
    for fee in claimable_fees {
        messages.push(fee.into_msg(info.sender.clone())?);
    }

    Ok(Response::new()
        .add_attributes(vec![("action", "claim")])
        .add_messages(messages))
}
