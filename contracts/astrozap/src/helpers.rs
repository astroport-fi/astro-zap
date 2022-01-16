use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Event, QuerierWrapper, QueryRequest, Reply, StdError,
    StdResult, SubMsg, SubMsgExecutionResponse, WasmMsg, WasmQuery,
};
use cw20::Cw20ExecuteMsg;

use cw_asset::{Asset, AssetInfo, AssetList};

use astroport::asset::PairInfo;
use astroport::pair::{ExecuteMsg, PoolResponse, SimulationResponse};

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
    sent_funds: &AssetList,
    sender_addr: &Addr,
    contract_addr: &Addr,
) -> StdResult<Option<CosmosMsg>> {
    match claimed_deposit.info {
        AssetInfo::Cw20(_) => Ok(Some(
            claimed_deposit.transfer_from_msg(sender_addr, contract_addr)?,
        )),
        AssetInfo::Native(_) => {
            let sent_fund = sent_funds.find(&claimed_deposit.info).ok_or_else(|| {
                StdError::generic_err(
                    format!("invalid deposit: expected {}, received none", claimed_deposit)
                )
            })?;
            if sent_fund != claimed_deposit {
                return Err(StdError::generic_err(
                    format!("invalid deposit: expected {}, received {}", claimed_deposit, sent_fund)
                ));
            }
            Ok(None)
        }
    }
}

// Handle multiple deposits by invoking `handle_deposit` on each of the claimed deposit
pub fn handle_deposits(
    claimed_deposits: &AssetList,
    sent_funds: &AssetList,
    sender_addr: &Addr,
    contract_addr: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    for deposit in claimed_deposits {
        if let Some(msg) = handle_deposit(&deposit, sent_funds, sender_addr, contract_addr)? {
            msgs.push(msg);
        }
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

/// Generate a submessage for swapping an asset using an Astroport pool
///
/// NOTE: We use reply_id: 0
pub fn build_swap_submsg(pair_addr: &Addr, offer_asset: &Asset) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_on_success(
        offer_asset.send_msg(
            pair_addr,
            to_binary(&astroport::pair::Cw20HookMsg::Swap {
                belief_price: None,
                max_spread: None,
                to: None,
            })?,
        )?,
        0,
    ))
}

/// Generate submessages for providing liqudity to an Astroport pool
///
/// NOTE: We use reply_id: 1
pub fn build_provide_liquidity_submsgs(
    pair_addr: &Addr,
    assets: &AssetList,
) -> StdResult<Vec<SubMsg>> {
    let mut submsgs: Vec<SubMsg> = vec![];
    let mut funds: Vec<Coin> = vec![];

    for asset in assets {
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
    }

    submsgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&ExecuteMsg::ProvideLiquidity {
                assets: assets.try_into_legacy()?,
                slippage_tolerance: None,
                auto_stake: None,
                receiver: None,
            })?,
            funds,
        },
        1,
    ));

    Ok(submsgs)
}
