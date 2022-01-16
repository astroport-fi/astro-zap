use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_asset::AssetList;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CacheData {
    pub user_addr: Addr,
    pub pair_addr: Addr,
    pub liquidity_token_addr: Addr,
    pub assets: AssetList,
    pub minimum_received: Option<Uint128>,
}

pub const CACHE: Item<CacheData> = Item::new("cache");
