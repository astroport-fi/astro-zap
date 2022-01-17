use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, Decimal, QuerierResult, SystemError};

use astroport::asset::{Asset as LegacyAsset, PairInfo};
use astroport::pair::{PoolResponse, QueryMsg, SimulationResponse};
use astroport_pair::contract::compute_swap;

#[derive(Default)]
pub struct PairQuerier {
    pair_infos: HashMap<Addr, PairInfo>,
    pool_infos: HashMap<Addr, PoolResponse>,
}

impl PairQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        match query {
            QueryMsg::Pair {} => self.query_pair(contract_addr),
            QueryMsg::Pool {} => self.query_pool(contract_addr),
            QueryMsg::Simulation { offer_asset } => self.query_simulation(contract_addr, offer_asset),

            q => Err(SystemError::UnsupportedRequest { kind: format!("[mock]: {:?}", q) }).into(),
        }
    }

    fn query_pair(&self, contract_addr: &Addr) -> QuerierResult {
        let pair_info = match self.pair_infos.get(contract_addr) {
            Some(pair_info) => pair_info,
            None => {
                return Err(SystemError::InvalidRequest {
                    error: format!("[mock]: pair info not set for pair {}", contract_addr.to_string()),
                    request: Default::default(),
                })
                .into();
            }
        };

        Ok(to_binary(&pair_info).into()).into()
    }

    fn query_pool(&self, contract_addr: &Addr) -> QuerierResult {
        let pool_info = match self.pool_infos.get(contract_addr) {
            Some(pool_info) => pool_info,
            None => {
                return Err(SystemError::InvalidRequest {
                    error: format!("[mock]: pool info not set for pair {}", contract_addr.to_string()),
                    request: Default::default(),
                })
                .into();
            }
        };

        Ok(to_binary(&pool_info).into()).into()
    }

    fn query_simulation(&self, contract_addr: &Addr, offer_asset: LegacyAsset) -> QuerierResult {
        let pool_info = match self.pool_infos.get(contract_addr) {
            Some(pool_info) => pool_info,
            None => {
                return Err(SystemError::InvalidRequest {
                    error: format!("[mock]: pool info not set for pair {}", contract_addr.to_string()),
                    request: Default::default(),
                })
                .into();
            }
        };
        let pools = pool_info.assets.clone();

        // Code below is copied from:
        // https://github.com/astroport-fi/astroport-core/blob/v1.0.1/contracts/pair/src/contract.rs#L881
        let offer_pool: LegacyAsset;
        let ask_pool: LegacyAsset;
        if offer_asset.info.equal(&pools[0].info) {
            offer_pool = pools[0].clone();
            ask_pool = pools[1].clone();
        } else if offer_asset.info.equal(&pools[1].info) {
            offer_pool = pools[1].clone();
            ask_pool = pools[0].clone();
        } else {
            return Err(SystemError::InvalidRequest {
                error: format!("[mock]: given offer asset doesn't belong to pairs"),
                request: Default::default(),
            })
            .into();
        }

        let total_fee_rate = Decimal::from_ratio(30u128, 10000u128); // 0.3%
        match compute_swap(
            offer_pool.amount,
            ask_pool.amount,
            offer_asset.amount,
            total_fee_rate,
        ) {
            Ok((return_amount, spread_amount, commission_amount)) => {
                Ok(to_binary(&SimulationResponse {
                    return_amount,
                    spread_amount,
                    commission_amount,
                })
                .into())
                .into()
            }
            Err(err) => Err(SystemError::InvalidRequest {
                error: format!("[mock]: failed to compute swap! reason: {}", err),
                request: Default::default(),
            })
            .into(),
        }
    }

    pub fn set_pair(&mut self, contract: &str, pair_info: PairInfo) {
        self.pair_infos.insert(Addr::unchecked(contract), pair_info);
    }

    pub fn set_pool(&mut self, contract: &str, pool_info: PoolResponse) {
        self.pool_infos.insert(Addr::unchecked(contract), pool_info);
    }
}
