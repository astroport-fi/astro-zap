use std::str::FromStr;

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Event, QuerierWrapper, QueryRequest, Reply,
    StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg, WasmQuery,
};
use cw20::Cw20ExecuteMsg;

use cw_asset::{Asset, AssetInfo, AssetList};
use cw_bigint::{BigInt, BigUint};

use astroport::asset::PairInfo;
use astroport::pair::{ExecuteMsg, PoolResponse, SimulationResponse, MAX_ALLOWED_SLIPPAGE};

const POW_32: u128 = 2u128.pow(32);

/// Convert a cw_bigint::BigUint to cosmwasm_std::Uint128
pub fn biguint_to_uint128(bui: &BigUint) -> StdResult<Uint128> {
    let digits = bui.to_u32_digits();
    let mut factor = Uint128::new(1u128);
    let mut ui = Uint128::zero();
    for digit in &digits {
        ui = ui.checked_add(Uint128::new(u128::from(*digit)).checked_mul(factor)?)?;
        factor = factor.checked_mul(Uint128::new(POW_32))?;
    }
    Ok(ui)
}

/// Convert a num_bigint::BigInt to cosmwasm_std::Uint128
pub fn bigint_to_uint128(bi: &BigInt) -> StdResult<Uint128> {
    biguint_to_uint128(
        &bi.to_biguint().ok_or_else(|| StdError::generic_err(format!("big int is negative: {}", bi)))?
    )
}

/// Extract response from reply
pub fn unwrap_reply(reply: Reply) -> StdResult<SubMsgExecutionResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

/// Determine if an event contains a specific key-value pair
pub fn event_contains_attr(event: &Event, key: &str, value: &str) -> bool {
    event
        .attributes
        .iter()
        .any(|attr| attr.key == key && attr.value == value)
}

/// Handle deposit:
/// - For CW20, draw the token from the sender's wallet, and return a `Some<CosmosMsg>`
/// - For native, assert the declared has indeed been transferred along with the message, return `None`
pub fn handle_deposit(
    claimed_deposit: &Asset,
    received_coins: &mut AssetList,
    sender_addr: &Addr,
    contract_addr: &Addr,
) -> StdResult<Option<CosmosMsg>> {
    match claimed_deposit.info {
        AssetInfo::Cw20(_) => Ok(Some(
            claimed_deposit.transfer_from_msg(sender_addr, contract_addr)?,
        )),
        AssetInfo::Native(_) => {
            // We need to have `received_coin` as a clone here to prevent the `cannot borrow as 
            // mutable because it is also borrowed as immutable` error
            let received_coin = received_coins
                .find(&claimed_deposit.info)
                .ok_or_else(|| StdError::generic_err(
                    format!("invalid deposit: expected {}, received none", claimed_deposit)
                ))?
                .clone();

            if received_coin != *claimed_deposit {
                return Err(StdError::generic_err(
                    format!("invalid deposit: expected {}, received {}", claimed_deposit, received_coin.amount)
                ));
            }

            received_coins.deduct(&received_coin)?;

            Ok(None)
        }
    }
}

// Handle multiple deposits by invoking `handle_deposit` on each of the claimed deposit
//
// This function takes a mutable asset list `received_coins` which is all the native tokens the user
// sent to the contract. For every native coin depsosit we processed, we deduct it from this list.
// At the end, we check whether this list is empty. If not, it means the user has sent extra funds,
// and we throw an error. 
pub fn handle_deposits(
    claimed_deposits: &AssetList,
    received_coins: &mut AssetList,
    sender_addr: &Addr,
    contract_addr: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    for deposit in claimed_deposits {
        if let Some(msg) = handle_deposit(deposit, received_coins, sender_addr, contract_addr)? {
            msgs.push(msg);
        }
    }

    if received_coins.len() > 0 {
        return Err(StdError::generic_err(
            format!("extra deposit received: {}", received_coins)
        ))
    }

    Ok(msgs)
}

/// Query an Astroport pair contract of its basic info
pub fn query_pair(querier: &QuerierWrapper, pair_addr: &Addr) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_addr.to_string(),
        msg: to_binary(&astroport::pair::QueryMsg::Pair {})?,
    }))
}

/// Query an Astroport pair contract of its pool info, specifically its asset depths and total
/// supply of its liquidity token
pub fn query_pool(querier: &QuerierWrapper, pair_addr: &Addr) -> StdResult<PoolResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_addr.to_string(),
        msg: to_binary(&astroport::pair::QueryMsg::Pool {})?,
    }))
}

/// Simulate the outcome of a swap
pub fn query_simulation(
    querier: &QuerierWrapper,
    pair_addr: &Addr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_addr.to_string(),
        msg: to_binary(&astroport::pair::QueryMsg::Simulation {
            offer_asset: offer_asset.into(),
        })?,
    }))
}

/// Generate a submessage for swapping an asset using an Astroport pool, and deduct the asset to be
/// offered from the list of available assets.
///
/// NOTE: 
/// 
/// - We use reply_id: 1
/// - We use Astroport's maximum allowed slippage. To limit slippage, the frontend should calculate
///   and supply the `minimum_received` parameter. 
pub fn build_swap_submsgs(
    pair_addr: &Addr, 
    available_assets: &mut AssetList, 
    offer_asset: &Asset,
) -> StdResult<Vec<SubMsg>> {
    let msg = match &offer_asset.info {
        AssetInfo::Cw20(_) => offer_asset.send_msg(
            pair_addr,
            to_binary(&astroport::pair::Cw20HookMsg::Swap {
                belief_price: None,
                max_spread: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
                to: None,
            })?,
        )?,
        AssetInfo::Native(denom) => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&ExecuteMsg::Swap {
                offer_asset: offer_asset.clone().into(),
                belief_price: None,
                max_spread: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
                to: None,
            })?,
            funds: vec![Coin {
                denom: denom.clone(),
                amount: offer_asset.amount,
            }],
        }),
    };

    available_assets.deduct(offer_asset)?;

    Ok(vec![SubMsg::reply_on_success(msg, 1)])
}

/// Generate submessages for providing liqudity to an Astroport pool, and deduct the assets to be
/// provided from the list of available assets.
///
/// NOTE: We use reply_id: 2
pub fn build_provide_liquidity_submsgs(
    pair_addr: &Addr,
    available_assets: &mut AssetList,
) -> StdResult<Vec<SubMsg>> {
    let mut submsgs: Vec<SubMsg> = vec![];
    let mut funds: Vec<Coin> = vec![];
    let mut assets_to_provide = AssetList::new();

    for asset in available_assets.clone().into_iter() {
        match &asset.info {
            AssetInfo::Cw20(contract_addr) => submsgs.push(SubMsg::new(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair_addr.to_string(),
                    amount: asset.amount,
                    expires: None,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(denom) => funds.push(Coin {
                denom: denom.clone(),
                amount: asset.amount,
            }),
        }

        available_assets.deduct(asset)?;
        assets_to_provide.add(asset)?;
    }

    submsgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&ExecuteMsg::ProvideLiquidity {
                assets: assets_to_provide.try_into_legacy()?,
                slippage_tolerance: None,
                auto_stake: None,
                receiver: None,
            })?,
            funds,
        },
        2,
    ));

    Ok(submsgs)
}
