use crate::contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::Config;
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{coins, Addr, Coin, Empty, Uint128};
use cw_multi_test::{
    custom_app, next_block, App, AppResponse, Contract, ContractWrapper, Executor,
};
use cwd_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use cwd_interface::Admin;

const DAO_ADDR: &str = "dao";
const VAULT_ADDR: &str = "vault";
const DESCRIPTION: &str = "description";
const NEW_DESCRIPTION: &str = "new description";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const DENOM: &str = "ujuno";
const INVALID_DENOM: &str = "uinvalid";
const INIT_BALANCE: Uint128 = Uint128::new(10000);

fn vault_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    custom_app(|r, _a, s| {
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(DAO_ADDR),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR1),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADDR2),
                vec![
                    Coin {
                        denom: DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                    Coin {
                        denom: INVALID_DENOM.to_string(),
                        amount: INIT_BALANCE,
                    },
                ],
            )
            .unwrap();
    })
}

fn instantiate_voting_registry(app: &mut App, id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(id, Addr::unchecked(DAO_ADDR), &msg, &[], "vault", None)
        .unwrap()
}

fn update_config(
    app: &mut App,
    contract_addr: Addr,
    sender: &str,
    owner: Option<String>,
    manager: Option<String>,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        contract_addr,
        &ExecuteMsg::UpdateConfig { owner, manager },
        &[],
    )
}

fn add_vault(
    app: &mut App,
    contract_addr: Addr,
    sender: &str,
    new_voting_vault_contract: String,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        contract_addr,
        &ExecuteMsg::AddVotingVault {
            new_voting_vault_contract,
        },
        &[],
    )
}

fn remove_vault(
    app: &mut App,
    contract_addr: Addr,
    sender: &str,
    old_voting_vault_contract: String,
) -> anyhow::Result<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        contract_addr,
        &ExecuteMsg::RemoveVotingVault {
            old_voting_vault_contract,
        },
        &[],
    )
}

fn get_config(app: &mut App, contract_addr: Addr) -> Config {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetConfig {})
        .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    // Populated fields
    let _addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::Address {
                addr: DAO_ADDR.to_string(),
            }),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    // Non populated fields
    let _addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: None,
            manager: None,
            voting_vault: VAULT_ADDR.to_string(),
        },
    );
}

#[test]
fn test_instantiate_dao_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    // Populated fields
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let config = get_config(&mut app, addr);

    assert_eq!(config.owner, Some(Addr::unchecked(DAO_ADDR)))
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_update_config_invalid_sender() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    // From ADDR2, so not owner or manager
    update_config(
        &mut app,
        addr,
        ADDR2,
        Some(ADDR1.to_string()),
        Some(DAO_ADDR.to_string()),
    )
    .unwrap();
}

#[test]
#[should_panic(expected = "Only owner can change owner")]
fn test_update_config_non_owner_changes_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    // ADDR1 is the manager so cannot change the owner
    update_config(&mut app, addr, ADDR1, Some(ADDR2.to_string()), None).unwrap();
}

#[test]
fn test_update_config_as_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    // Swap owner and manager, change description
    update_config(
        &mut app,
        addr.clone(),
        DAO_ADDR,
        Some(ADDR1.to_string()),
        Some(DAO_ADDR.to_string()),
    )
    .unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        Config {
            owner: Some(Addr::unchecked(ADDR1)),
            manager: Some(Addr::unchecked(DAO_ADDR)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR)]
        },
        config
    );
}

#[test]
fn test_update_config_as_manager() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    // Change description and manager as manager cannot change owner
    update_config(
        &mut app,
        addr.clone(),
        ADDR1,
        Some(DAO_ADDR.to_string()),
        Some(ADDR2.to_string()),
    )
    .unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR2)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR)]
        },
        config
    );
}

#[test]
fn test_query_dao() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let msg = QueryMsg::Dao {};
    let dao: Addr = app.wrap().query_wasm_smart(addr, &msg).unwrap();
    assert_eq!(dao, Addr::unchecked(DAO_ADDR));
}

#[test]
fn test_query_info() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let msg = QueryMsg::Info {};
    let resp: InfoResponse = app.wrap().query_wasm_smart(addr, &msg).unwrap();
    assert_eq!(resp.info.contract, "crates.io:neutron-voting-registry");
}

#[test]
fn test_query_get_config() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let config = get_config(&mut app, addr);
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR)]
        }
    )
}

#[test]
fn test_add_vault_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let new_vault: &str = "new_vault";
    add_vault(&mut app, addr.clone(), DAO_ADDR, new_vault.to_string()).unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR), Addr::unchecked(new_vault)]
        }
    )
}

#[test]
fn test_remove_vault_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let new_vault: &str = "new_vault";
    add_vault(&mut app, addr.clone(), DAO_ADDR, new_vault.to_string()).unwrap();

    let config = get_config(&mut app, addr.clone());
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR), Addr::unchecked(new_vault)]
        }
    );

    remove_vault(&mut app, addr.clone(), DAO_ADDR, VAULT_ADDR.to_string()).unwrap();
    let config = get_config(&mut app, addr);
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            voting_vaults: vec![Addr::unchecked(new_vault)]
        }
    );
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_add_vault_manager() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let new_vault: &str = "new_vault";
    add_vault(&mut app, addr.clone(), ADDR1, new_vault.to_string()).unwrap();

    let config = get_config(&mut app, addr);
    assert_eq!(
        config,
        Config {
            owner: Some(Addr::unchecked(DAO_ADDR)),
            manager: Some(Addr::unchecked(ADDR1)),
            voting_vaults: vec![Addr::unchecked(VAULT_ADDR)]
        }
    )
}

#[test]
#[should_panic(expected = "Removing last vault is forbidden")]
fn test_remove_last_vault_owner() {
    let mut app = mock_app();
    let vault_id = app.store_code(vault_contract());
    let addr = instantiate_voting_registry(
        &mut app,
        vault_id,
        InstantiateMsg {
            owner: Some(Admin::CoreModule {}),
            manager: Some(ADDR1.to_string()),
            voting_vault: VAULT_ADDR.to_string(),
        },
    );

    let new_vault: &str = "new_vault";
    remove_vault(&mut app, addr.clone(), DAO_ADDR, new_vault.to_string()).unwrap();
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
