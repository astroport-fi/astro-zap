use std::str::FromStr;

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsgExecutionResponse, Uint128,
};

use astroport::factory::PairType;

use cw_asset::{Asset, AssetInfo, AssetList};

use crate::helpers::{
    build_provide_liquidity_submsgs, build_swap_submsgs, event_contains_attr, handle_deposits,
    query_pair, query_pool, query_simulation, unwrap_reply, bigint_to_uint128
};
use crate::math::Quadratic;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SimulateEnterResponse};
use crate::state::{CacheData, CACHE};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new()) // do nothing
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Enter {
            pair,
            deposits,
            minimum_received,
        } => enter(
            deps,
            env,
            info,
            api.addr_validate(&pair)?,
            deposits.check(api, None)?,
            minimum_received,
        ),
    }
}

fn enter(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_addr: Addr,
    mut deposits: AssetList,
    minimum_received: Option<Uint128>,
) -> StdResult<Response> {
    let pair_info = query_pair(&deps.querier, &pair_addr)?;
    let pool_info = query_pool(&deps.querier, &pair_addr)?;
    let pool_assets = AssetList::from_legacy(&pool_info.assets);

    // The pair must be of xyz type
    assert_pair_type(&pair_info.pair_type)?;
    // Each deposited asset must be contained by the pool
    assert_deposit_types(&pool_assets, &deposits)?;
    // Must deposit exactly 1 or 2 non-zero assets
    deposits.purge();
    assert_deposit_number(&deposits)?;

    // Handle deposits
    // If the user claims to have deposited a CW20 token, we draw it from the user's wallet (user
    // must have approved allowance)
    // If the user claims to have deposited a native coin, we assert that the exact coin was indeed
    // sent alone with `info.funds`
    let deposit_msgs = handle_deposits(
        &deposits,
        &mut info.funds.into(),
        &info.sender,
        &env.contract.address,
    )?;

    // Compute the optimal swap that will yield the most liquidity tokens, and deduct the amount
    // that will be sent out from available assets
    // Then, deduct the offer asset from the user's available assets (as they will be sent out)
    let offer_asset = compute_offer_asset(&pool_assets, &deposits)?;

    // Build submsgs
    //
    // If no swap is needed (i.e. offer amount is calculated to be zero), we simply provide the
    // liquidity, and deduct the assets to be provided from the list of available assets
    //
    // If a swap is needed, we execute the swap, and deduct the offer asset from the list of
    // available assets
    let submsgs = if offer_asset.amount.is_zero() {
        build_provide_liquidity_submsgs(&pair_addr, &mut deposits)?
    } else {
        build_swap_submsgs(&pair_addr, &mut deposits, &offer_asset)?
    };

    // Cache necessary data so that they can be accessed when handling reply
    let cache = CacheData {
        user_addr: info.sender,
        pair_addr: pair_addr.clone(),
        liquidity_token_addr: pair_info.liquidity_token,
        assets: deposits.clone(),
        minimum_received,
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_messages(deposit_msgs)
        .add_submessages(submsgs)
        .add_attribute("action", "astrozap/execute/enter")
        .add_attribute("assets_deposited", deposits.to_string()))
}

/// Assert the given Astroport pair is of the XYK type
fn assert_pair_type(pair_type: &PairType) -> StdResult<()> {
     match pair_type {
         PairType::Xyk {} => Ok(()),
         pt => Err(StdError::generic_err(format!("unsupported pair type: {}", pt))),
     }
}

/// Assert each of the deposited asset must be contained by the Astroport pair
fn assert_deposit_types(pair_assets: &AssetList, deposits: &AssetList) -> StdResult<()> {
    for deposit in deposits {
        if pair_assets.find(&deposit.info).is_none() {
            return Err(StdError::generic_err(
                format!("pair does not contain asset {}", deposit.info)
            ));
        }
    }
    Ok(())
}

/// Assert that deposits must contain either exactly one or two assets
fn assert_deposit_number(deposits: &AssetList) -> StdResult<()> {
    if !(1..=2).contains(&deposits.len()) {
        return Err(StdError::generic_err(
            format!("must deposit exactly 1 or 2 assets; received {}", deposits.len())
        ));
    }
    Ok(())
}

/// Compute the maximal amount of asset to swap such that providing the two assets afterwards will
/// return the greatest amount of liquidity tokens
///
/// For details of the math involved, see `../../docs/astrozap.pdf`
fn compute_offer_asset(pool_assets: &AssetList, user_assets: &AssetList) -> StdResult<Asset> {
    let a_pool = pool_assets[0].clone();
    let b_pool = pool_assets[1].clone();

    let a_user = user_assets
        .find(&a_pool.info)
        .cloned()
        .unwrap_or_else(|| Asset::new(a_pool.info.clone(), 0u128));
    let b_user = user_assets
        .find(&b_pool.info)
        .cloned()
        .unwrap_or_else(|| Asset::new(b_pool.info.clone(), 0u128));

    // Compute which asset the user has a bigger share; we swap the asset with the bigger share into
    // the one with the smaller share
    let share_a = Decimal256::from_ratio(a_user.amount, a_pool.amount);
    let share_b = Decimal256::from_ratio(b_user.amount, b_pool.amount);

    let q = if share_a > share_b {
        Quadratic::from_asset_amounts(
            &a_user.amount.u128().into(),
            &a_pool.amount.u128().into(),
            &b_user.amount.u128().into(),
            &b_pool.amount.u128().into(),
        )
    } else {
        Quadratic::from_asset_amounts(
            &b_user.amount.u128().into(),
            &b_pool.amount.u128().into(),
            &a_user.amount.u128().into(),
            &a_pool.amount.u128().into(),
        )
    };

    // Solve quadratic equation to find out the swap amount
    //
    // Here we use 0 as the initial value. It is possible to find a better guess, but in experience
    // the equation usually converges in 4 - 5 iterations even starting with 0, so I'll go with this
    let offer_amount = bigint_to_uint128(&q.solve())?;

    let offer_asset_info = if share_a > share_b {
        a_pool.info
    } else {
        b_pool.info
    };

    Ok(Asset::new(offer_asset_info, offer_amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => after_swap(deps, unwrap_reply(reply)?),
        2 => after_provide_liquidity(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}", id))),
    }
}

fn after_swap(deps: DepsMut, res: SubMsgExecutionResponse) -> StdResult<Response> {
    let event = res
        .events
        .iter()
        .find(|event| event_contains_attr(event, "action", "swap"))
        .ok_or_else(|| StdError::generic_err("cannot find `swap` event"))?;

    let ask_asset_str = event
        .attributes
        .iter()
        .cloned()
        .find(|attr| attr.key == "ask_asset")
        .ok_or_else(|| StdError::generic_err("cannot find `ask_asset` attribute"))?
        .value;

    let return_amount_str = event
        .attributes
        .iter()
        .cloned()
        .find(|attr| attr.key == "return_amount")
        .ok_or_else(|| StdError::generic_err("cannot find `return_amount` attribute"))?
        .value;

    // If `ask_asset_str` can be validated as a Terra address, then we assume it is a CW20;
    // otherwise we assume it is a native coin
    let returned_info = if let Ok(contract_addr) = deps.api.addr_validate(&ask_asset_str) {
        AssetInfo::cw20(contract_addr)
    } else {
        AssetInfo::native(ask_asset_str)
    };
    let returned_amount = Uint128::from_str(&return_amount_str)?;
    let returned_asset = Asset::new(returned_info, returned_amount);

    let mut cache = CACHE.load(deps.storage)?;
    cache.assets.add(&returned_asset)?;

    // Build messages to provide assets to the DEX pool, and deduct the assets to be provided from
    // the list of available assets
    let submsgs = build_provide_liquidity_submsgs(
        &cache.pair_addr,
        &mut cache.assets,
    )?;
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_submessages(submsgs)
        .add_attribute("action", "astrozap/reply/after_swap")
        .add_attribute("asset_returned", returned_asset.to_string()))
}

fn after_provide_liquidity(deps: DepsMut, res: SubMsgExecutionResponse) -> StdResult<Response> {
    let event = res
        .events
        .iter()
        .find(|event| event_contains_attr(event, "action", "provide_liquidity"))
        .ok_or_else(|| StdError::generic_err("cannot find `provide_liquidity` event"))?;

    let share_str = event
        .attributes
        .iter()
        .cloned()
        .find(|attr| attr.key == "share")
        .ok_or_else(|| StdError::generic_err("cannot find `share` attribute"))?
        .value;

    let share_amount = Uint128::from_str(&share_str)?;

    let mut cache = CACHE.load(deps.storage)?;
    CACHE.remove(deps.storage);

    if let Some(minimum_received) = cache.minimum_received {
        if share_amount < minimum_received {
            return Err(StdError::generic_err(
                format!("too little received! minimum: {}, received {}", minimum_received, share_amount)
            ));
        }
    }

    let shares_minted = Asset::cw20(cache.liquidity_token_addr, share_amount);
    cache.assets.add(&shares_minted)?;

    Ok(Response::new()
        .add_messages(cache.assets.transfer_msgs(&cache.user_addr)?)
        .add_attribute("action", "astrozap/reply/after_providing_liquidity")
        .add_attribute("shares_minted", shares_minted.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;
    match msg {
        QueryMsg::SimulateEnter { pair, deposits } => to_binary(&query_simulate_enter(
            deps,
            api.addr_validate(&pair)?,
            deposits.check(api, None)?,
        )?),
    }
}

fn query_simulate_enter(
    deps: Deps,
    pair_addr: Addr,
    mut deposits: AssetList,
) -> StdResult<SimulateEnterResponse> {
    let pair_info = query_pair(&deps.querier, &pair_addr)?;
    let pool_info = query_pool(&deps.querier, &pair_addr)?;
    let mut pool_assets = AssetList::from_legacy(&pool_info.assets);

    // The pair must be of xyz type
    assert_pair_type(&pair_info.pair_type)?;
    // Each deposited asset must be contained by the pool
    assert_deposit_types(&pool_assets, &deposits)?;
    // Must deposit exactly 1 or 2 non-zero assets
    deposits.purge();
    assert_deposit_number(&deposits)?;

    let offer_asset = compute_offer_asset(&pool_assets, &deposits)?;

    let simulation = query_simulation(&deps.querier, &pair_addr, &offer_asset)?;
    let return_info = if offer_asset.info == pool_assets[0].info {
        pool_assets[1].info.clone()
    } else {
        pool_assets[0].info.clone()
    };
    let return_asset = Asset::new(return_info, simulation.return_amount);

    pool_assets.add(&offer_asset)?;
    pool_assets.deduct(&return_asset)?;

    deposits.add(&return_asset)?;
    deposits.deduct(&offer_asset)?;

    // https://github.com/astroport-fi/astroport-core/blob/master/contracts/pair/src/contract.rs#L386
    let mint_shares = std::cmp::min(
        deposits
            .find(&pool_assets[0].info)
            .map(|asset| asset.amount)
            .unwrap_or_else(Uint128::zero)
            .multiply_ratio(pool_info.total_share, pool_assets[0].amount),
        deposits
            .find(&pool_assets[1].info)
            .map(|asset| asset.amount)
            .unwrap_or_else(Uint128::zero)
            .multiply_ratio(pool_info.total_share, pool_assets[1].amount),
    );

    Ok(SimulateEnterResponse {
        offer_asset: offer_asset.into(),
        return_asset: return_asset.into(),
        mint_shares,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new()) // do nothing
}
