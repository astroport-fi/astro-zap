use cosmwasm_std::testing::MockQuerier;
use cosmwasm_std::{
    from_binary, from_slice, Addr, Empty, Querier, QuerierResult, QueryRequest, StdResult,
    SystemError, WasmQuery,
};

use astroport::asset::PairInfo;
use astroport::pair::PoolResponse;

use super::pair_querier::PairQuerier;

// We do not have any custom query
type CustomQuery = Empty;

pub struct CustomMockQuerier {
    base: MockQuerier<CustomQuery>,
    pair_querier: PairQuerier,
}

impl Default for CustomMockQuerier {
    fn default() -> Self {
        Self {
            base: MockQuerier::<CustomQuery>::new(&[]),
            pair_querier: PairQuerier::default(),
        }
    }
}

impl Querier for CustomMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<CustomQuery> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("[mock]: failed to parse query request {}", e),
                    request: bin_request.into(),
                })
                .into()
            }
        };
        self.handle_query(&request)
    }
}

impl CustomMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<CustomQuery>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                let contract_addr = Addr::unchecked(contract_addr);

                let parse_pair_query: StdResult<astroport::pair::QueryMsg> = from_binary(msg);
                if let Ok(pair_query) = parse_pair_query {
                    return self.pair_querier.handle_query(&contract_addr, pair_query);
                }

                panic!("[mock]: failed to parse wasm query {:?}", msg)
            }

            _ => self.base.handle_query(request),
        }
    }

    pub fn set_pair(&mut self, contract: &str, pair_info: PairInfo) {
        self.pair_querier.set_pair(contract, pair_info);
    }

    pub fn set_pool(&mut self, contract: &str, pool_info: PoolResponse) {
        self.pair_querier.set_pool(contract, pool_info);
    }
}
