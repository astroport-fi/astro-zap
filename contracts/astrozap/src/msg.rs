use cosmwasm_std::{Empty, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_asset::{AssetUnchecked, AssetListUnchecked};

/// We currently don't need any parameter for instantiation and migration
pub type InstantiateMsg = Empty;
pub type MigrateMsg = Empty;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Provide specified assets to a pair
    ///
    /// NOTE:
    ///
    /// - For CW20 tokens, the sender must have approved allowance. For native coins, the exact
    /// amount must be sent along with the message
    ///
    /// - The frontend should calculate `minimum_received` and supply it as an input paramter
    Enter {
        pair: String,
        deposits: AssetListUnchecked,
        minimum_received: Option<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Compute the amount of liquidity tokens that will be minted by executing the `Enter` command
    /// with the given assets. Returns `SimulateResponse`
    SimulateEnter {
        pair: String,
        deposits: AssetListUnchecked,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulateResponse {
    /// The asset that will be offered for swap in order to balance the values or the two assets
    pub offer_asset: AssetUnchecked,
    /// The asset that will be returned as the result of swapping `offer_asset`
    pub return_asset: AssetUnchecked,
    /// The amount of liquidity tokens that will be minted by providing the two assets after the swap
    pub mint_shares: Uint128,
}
