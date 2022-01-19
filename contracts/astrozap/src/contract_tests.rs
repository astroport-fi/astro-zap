use cosmwasm_std::testing::{mock_env, mock_info, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Coin, ContractResult, CosmosMsg, Event, OwnedDeps, Reply,
    ReplyOn, StdError, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg, Decimal
};

use cw_asset::{Asset, AssetInfo, AssetList};

use astroport::asset::PairInfo;
use astroport::factory::PairType;
use astroport::pair::PoolResponse;

use crate::contract::{execute, query, reply};
use crate::msg::{ExecuteMsg, QueryMsg, SimulateEnterResponse};
use crate::state::{CacheData, CACHE};
use crate::test_utils::{mock_dependencies, CustomMockApi, CustomMockQuerier};

fn setup_test() -> OwnedDeps<MockStorage, CustomMockApi, CustomMockQuerier> {
    let mut deps = mock_dependencies();

    deps.querier.set_pair(
        "luna_ust_pair",
        PairInfo {
            asset_infos: [
                AssetInfo::native("uusd").into(),
                AssetInfo::native("uluna").into(),
            ],
            contract_addr: Addr::unchecked("luna_ust_pair"),
            liquidity_token: Addr::unchecked("luna_ust_lp_token"),
            pair_type: PairType::Xyk {},
        },
    );
    deps.querier.set_pool(
        "luna_ust_pair",
        PoolResponse {
            assets: [
                Asset::native("uusd", 118070429547232u128).into(),
                Asset::native("uluna", 1451993415113u128).into(),
            ],
            total_share: Uint128::new(12966110801826u128),
        },
    );

    deps.querier.set_pair(
        "astro_ust_pair",
        PairInfo {
            asset_infos: [
                AssetInfo::cw20(Addr::unchecked("astro_token")).into(),
                AssetInfo::native("uusd").into(),
            ],
            contract_addr: Addr::unchecked("astro_ust_pair"),
            liquidity_token: Addr::unchecked("astro_ust_lp_token"),
            pair_type: PairType::Xyk {},
        },
    );
    deps.querier.set_pool(
        "astro_ust_pair",
        PoolResponse {
            assets: [
                Asset::cw20(Addr::unchecked("astro_token"), 48059201882191u128).into(),
                Asset::native("uusd", 65155920988539u128).into(),
            ],
            total_share: Uint128::new(55851193190261u128),
        },
    );

    deps.querier.set_pair(
        "bluna_luna_pair",
        PairInfo {
            asset_infos: [
                AssetInfo::cw20(Addr::unchecked("bluna_token")).into(),
                AssetInfo::native("uluna").into(),
            ],
            contract_addr: Addr::unchecked("bluna_luna_pair"),
            liquidity_token: Addr::unchecked("bluna_luna_lp_token"),
            pair_type: PairType::Stable {},
        },
    );
    deps.querier.set_pool(
        "bluna_luna_pair",
        PoolResponse {
            assets: [
                Asset::cw20(Addr::unchecked("bluna_token"), 2961459937027u128).into(),
                Asset::native("uluna", 2937863752918u128).into(),
            ],
            total_share: Uint128::new(2948589474051u128),
        },
    );

    deps
}

#[test]
fn should_reject_wrong_pair_type() {
    let mut deps = setup_test();

    let msg = ExecuteMsg::Enter {
        pair: String::from("bluna_luna_pair"),
        deposits: AssetList::from(vec![
            Asset::cw20(Addr::unchecked("bluna_token"), 12345u128),
            Asset::native("uluna", 12345u128),
        ])
        .into(),
        minimum_received: None,
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info("alice", &[]), msg);
    assert_eq!(
        err,
        Err(StdError::generic_err("unsupported pair type: stable"))
    );
}

#[test]
fn should_reject_wrong_deposit_type() {
    let mut deps = setup_test();

    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![
            Asset::cw20(Addr::unchecked("astro_token"), 12345u128),
            Asset::native("uusd", 12345u128),
        ])
        .into(),
        minimum_received: None,
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info("alice", &[]), msg);
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "pair does not contain asset cw20:astro_token"
        ))
    );
}

#[test]
fn should_reject_wrong_deposit_number() {
    let mut deps = setup_test();

    // Deposit no asset
    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![Asset::native("uluna", 0u128)]).into(),
        minimum_received: None,
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info("alice", &[]), msg);
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "must deposit exactly 1 or 2 assets; received 0"
        ))
    );

    // Deposit three assets
    //
    // This is actually an valid input, since it contains two non-zero assets. However we still
    // considers it a malformed input and throws an error.
    //
    // Later we may update the `purge` function to also merge duplicate assets, in which case this
    // input will be considered legal
    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![
            Asset::native("uluna", 12345u128),
            Asset::native("uusd", 12345u128),
            Asset::native("uluna", 12345u128),
        ])
        .into(),
        minimum_received: None,
    };
    let err = execute(deps.as_mut(), mock_env(), mock_info("alice", &[]), msg);
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "must deposit exactly 1 or 2 assets; received 3"
        ))
    );
}

#[test]
fn should_reject_missing_deposit() {
    let mut deps = setup_test();

    // User claims to deposit 12345 uluna, but doesn't actually send any with the message
    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![Asset::native("uluna", 12345u128)]).into(),
        minimum_received: None,
    };
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[]),
        msg.clone(),
    );
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "invalid deposit: expected native:uluna:12345, received none"
        ))
    );
    // User claims to deposit 12345 uluna, but sends a different amount
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[Coin::new(88888, "uluna")]),
        msg.clone(),
    );
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "invalid deposit: expected native:uluna:12345, received 88888"
        ))
    );
}

#[test]
fn should_enter_native_native_pool() {
    let mut deps = setup_test();

    // Compute offer amount
    //
    // Using `scripts/math.ts`, create `Equation` instance using the following parameters:
    // offer_user = 100000000000
    // offer_pool = 118070429547232
    // ask_user = 0
    // ask_pool = 1451993415113
    //
    // Should calculate:
    // a = 1451993415113
    // b = 171807034937323920678165568
    // c = -17143748622214425401611721600000000000
    //
    // In 4 interations, should find solution: offer_amount = 50064546170
    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![Asset::native("uusd", 100000000000u128)]).into(),
        minimum_received: None,
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[Coin::new(100000000000, "uusd")]),
        msg,
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("luna_ust_pair"),
                msg: to_binary(&astroport::pair::ExecuteMsg::Swap {
                    offer_asset: Asset::native("uusd", 50064546170u128).into(),
                    belief_price: None,
                    max_spread: Some(Decimal::from_ratio(1u128, 2u128)),
                    to: None,
                })
                .unwrap(),
                funds: vec![Coin::new(50064546170, "uusd")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        }
    );

    // Using `scripts/cfmm.ts` to estimate:
    // offering 50064546170 uusd, should receive 613571013 uluna after commission
    // uusd available: 100000000000 - 50064546170 = 49935453830
    let _reply = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![Event::new("wasm")
                .add_attribute("action", "swap")
                .add_attribute("ask_asset", "uluna")
                .add_attribute("return_amount", "613571013")],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), _reply).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 2,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("luna_ust_pair"),
                msg: to_binary(&astroport::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset::native("uusd", 49935453830u128).into(),
                        Asset::native("uluna", 613571013u128).into(),
                    ],
                    slippage_tolerance: None,
                    auto_stake: None,
                    receiver: None
                })
                .unwrap(),
                funds: vec![
                    Coin::new(49935453830, "uusd"),
                    Coin::new(613571013, "uluna")
                ]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success
        }
    );

    // The pool should have:
    // uusd: 118070429547232 + 50064546170 = 118120494093402
    // uluna: 1451993415113 - 613571013 = 1451379844100
    // uLP: 12966110801826
    //
    // Amount of liquidity tokens to be minted:
    // min(
    //     12966110801826 * 49935453830 / 118120494093402,
    //     12966110801826 * 613571013 / 1451379844100,
    // )
    // = min(5481424982, 5481424984)
    // = 5481424982
    //
    // Should generate a message to refund
    let _reply = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![Event::new("wasm")
                .add_attribute("action", "provide_liquidity")
                .add_attribute("share", "5481424982")],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), _reply).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("luna_ust_lp_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(5481424982)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn should_enter_cw20_native_pool() {
    let mut deps = setup_test();

    // Compute offer amount
    //
    // Using `scripts/math.ts`, create `Equation` instance using the following parameters:
    // offer_user = 750000000000
    // offer_pool = 48059201882191
    // ask_user = 100000000000
    // ask_pool = 65155920988539
    //
    // Should calculate:
    // a = 65255920988539
    // b = 6262754336088952322758000272
    // c = -2117537481900892096900110663650000000000
    //
    // In 4 interations, should find solution: offer_amount = 336933122413
    let msg = ExecuteMsg::Enter {
        pair: String::from("astro_ust_pair"),
        deposits: AssetList::from(vec![
            Asset::native("uusd", 100000000000u128),
            Asset::cw20(Addr::unchecked("astro_token"), 750000000000u128), // ~$1M
        ])
        .into(),
        minimum_received: None,
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[Coin::new(100000000000, "uusd")]),
        msg,
    )
    .unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("astro_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: String::from("alice"),
                    recipient: String::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128::new(750000000000),
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("astro_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                    contract: String::from("astro_ust_pair"),
                    amount: Uint128::new(336933122413),
                    msg: to_binary(&astroport::pair::Cw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: Some(Decimal::from_ratio(1u128, 2u128)),
                        to: None,
                    })
                    .unwrap()
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        }
    );

    // Using `scripts/cfmm.ts` to estimate:
    // offering 336933122413 uASTRO, should receive 452253642498 uusd after commission
    // uASTRO available: 750000000000 - 336933122413 = 413066877587
    // uusd available: 100000000000 + 452253642498 = 552253642498
    let _reply = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![Event::new("wasm")
                .add_attribute("action", "swap")
                .add_attribute("ask_asset", "uusd")
                .add_attribute("return_amount", "452253642498")],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), _reply).unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("astro_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: String::from("astro_ust_pair"),
                    amount: Uint128::new(413066877587),
                    expires: None,
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 2,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("astro_ust_pair"),
                msg: to_binary(&astroport::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset::native("uusd", 552253642498u128).into(),
                        Asset::cw20(Addr::unchecked("astro_token"), 413066877587u128).into(),
                    ],
                    slippage_tolerance: None,
                    auto_stake: None,
                    receiver: None
                })
                .unwrap(),
                funds: vec![Coin::new(552253642498, "uusd"),]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success
        }
    );

    // The pool should have:
    // uASTRO: 48059201882191 + 336933122413 = 48396135004604
    // uusd: 65155920988539 - 452253642498 = 64702289434651
    // uLP: 55851193190261
    //
    // Amount of liquidity tokens to be minted:
    // min(
    //     55851193190261 * 413066877587 / 48396135004604,
    //     55851193190261 * 552253642498 / 64703667346041,
    // )
    // = min(476696702710, 476696702711)
    // = 476696702710
    //
    // Should generate a message to refund
    let _reply = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![Event::new("wasm")
                .add_attribute("action", "provide_liquidity")
                .add_attribute("share", "476696702710")],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), _reply).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("astro_ust_lp_token"),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(476696702710)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn should_enter_with_equal_value_assets() {
    let mut deps = setup_test();

    // We provide LUNA + UST that's exactly the pool depth
    // No swap should be needed
    let msg = ExecuteMsg::Enter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![
            Asset::native("uusd", 118070429547232u128),
            Asset::native("uluna", 1451993415113u128),
        ])
        .into(),
        minimum_received: None,
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(
            "alice",
            &[
                Coin::new(118070429547232, "uusd"),
                Coin::new(1451993415113, "uluna"),
            ],
        ),
        msg,
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 2,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("luna_ust_pair"),
                msg: to_binary(&astroport::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset::native("uusd", 118070429547232u128).into(),
                        Asset::native("uluna", 1451993415113u128).into(),
                    ],
                    slippage_tolerance: None,
                    auto_stake: None,
                    receiver: None
                })
                .unwrap(),
                funds: vec![
                    Coin::new(118070429547232, "uusd"),
                    Coin::new(1451993415113, "uluna")
                ]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success
        }
    );
}

#[test]
fn should_reject_excessive_slippage() {
    let mut deps = setup_test();

    CACHE
        .save(
            deps.as_mut().storage,
            &CacheData {
                user_addr: Addr::unchecked("alice"),
                pair_addr: Addr::unchecked("luna_ust_pair"),
                liquidity_token_addr: Addr::unchecked("luna_ust_lp_token"),
                assets: AssetList::default(),
                minimum_received: Some(Uint128::new(20000)),
            },
        )
        .unwrap();

    let _reply = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![Event::new("wasm")
                .add_attribute("action", "provide_liquidity")
                .add_attribute("share", "12345")],
            data: None,
        }),
    };
    let err = reply(deps.as_mut(), mock_env(), _reply);
    assert_eq!(
        err,
        Err(StdError::generic_err(
            "too little received! minimum: 20000, received 12345"
        ))
    );
}

#[test]
fn should_query_simulate() {
    let deps = setup_test();

    let msg = QueryMsg::SimulateEnter {
        pair: String::from("luna_ust_pair"),
        deposits: AssetList::from(vec![Asset::native("uusd", 100000000000u128)]).into(),
    };
    let res: SimulateEnterResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateEnterResponse {
            offer_asset: Asset::native("uusd", 50064546170u128).into(),
            return_asset: Asset::native("uluna", 613571013u128).into(),
            mint_shares: Uint128::new(5481424982)
        }
    );

    let msg = QueryMsg::SimulateEnter {
        pair: String::from("astro_ust_pair"),
        deposits: AssetList::from(vec![
            Asset::native("uusd", 100000000000u128),
            Asset::cw20(Addr::unchecked("astro_token"), 750000000000u128), // ~$1M
        ])
        .into(),
    };
    let res: SimulateEnterResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateEnterResponse {
            offer_asset: Asset::cw20(Addr::unchecked("astro_token"), 336933122413u128).into(),
            return_asset: Asset::native("uusd", 452253642498u128).into(),
            mint_shares: Uint128::new(476696702710)
        }
    );
}
