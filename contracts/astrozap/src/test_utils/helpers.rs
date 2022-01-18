use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::OwnedDeps;

use super::custom_mock_querier::CustomMockQuerier;
use super::custom_mock_api::CustomMockApi;

pub fn mock_dependencies() -> OwnedDeps<MockStorage, CustomMockApi, CustomMockQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: CustomMockApi::default(),
        querier: CustomMockQuerier::default(),
    }
}