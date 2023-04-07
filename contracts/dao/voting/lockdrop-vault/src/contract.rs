#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Fraction, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cwd_interface::voting::{
    BondingStatusResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use cwd_interface::Admin;
use neutron_lockdrop_vault::voting_power::{get_voting_power_for_address, get_voting_power_total};

use crate::state::{CONFIG, DAO};

use astroport_periphery::lockdrop::PoolType;
use cwd_voting::vault::ListBondersResponse;
use neutron_lockdrop_vault::error::ContractError;
use neutron_lockdrop_vault::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use neutron_lockdrop_vault::types::Config;

pub(crate) const CONTRACT_NAME: &str = "crates.io:neutron-lockdrop-vault";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = match msg.owner {
        Admin::Address { addr } => deps.api.addr_validate(addr.as_str())?,
        Admin::CoreModule {} => info.sender.clone(),
    };
    let manager = msg
        .manager
        .map(|manager| deps.api.addr_validate(&manager))
        .transpose()?;

    let config = Config {
        name: msg.name,
        description: msg.description,
        lockdrop_contract: deps.api.addr_validate(&msg.lockdrop_contract)?,
        oracle_contract: deps.api.addr_validate(&msg.oracle_contract)?,
        owner,
        manager,
    };
    config.validate()?;
    CONFIG.save(deps.storage, &config)?;
    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("name", config.name)
        .add_attribute("description", config.description)
        .add_attribute("owner", config.owner)
        .add_attribute("lockdrop_contract", config.lockdrop_contract)
        .add_attribute("oracle_contract", config.oracle_contract)
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond {} => execute_bond(deps, env, info),
        ExecuteMsg::Unbond { amount } => execute_unbond(deps, env, info, amount),
        ExecuteMsg::UpdateConfig {
            owner,
            lockdrop_contract,
            oracle_contract,
            manager,
            name,
            description,
        } => execute_update_config(
            deps,
            info,
            owner,
            lockdrop_contract,
            oracle_contract,
            manager,
            name,
            description,
        ),
    }
}

pub fn execute_bond(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    Err(ContractError::BondingDisabled {})
}

pub fn execute_unbond(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _amount: Uint128,
) -> Result<Response, ContractError> {
    Err(ContractError::DirectUnbondingDisabled {})
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    new_lockdrop_contract: Option<String>,
    new_oracle_contract: Option<String>,
    new_manager: Option<String>,
    new_name: Option<String>,
    new_description: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;

    let new_lockdrop_contract = new_lockdrop_contract
        .map(|new_lockdrop_contract| deps.api.addr_validate(&new_lockdrop_contract))
        .transpose()?;

    let new_oracle_contract = new_oracle_contract
        .map(|new_oracle_contract| deps.api.addr_validate(&new_oracle_contract))
        .transpose()?;

    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&new_manager))
        .transpose()?;

    if info.sender != config.owner && new_owner != Some(config.owner.clone()) {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };
    if info.sender != config.owner
        && Some(config.lockdrop_contract.clone()) != new_lockdrop_contract
    {
        return Err(ContractError::OnlyOwnerCanChangeLockdropContract {});
    };
    if info.sender != config.owner && Some(config.oracle_contract.clone()) != new_oracle_contract {
        return Err(ContractError::OnlyOwnerCanChangeOracleContract {});
    };

    config.manager = new_manager;

    if let Some(owner) = new_owner {
        config.owner = owner;
    }

    if let Some(lockdrop_contract) = new_lockdrop_contract {
        config.lockdrop_contract = lockdrop_contract;
    }
    if let Some(oracle_contract) = new_oracle_contract {
        config.oracle_contract = oracle_contract;
    }
    if let Some(name) = new_name {
        config.name = name;
    }
    if let Some(description) = new_description {
        config.description = description;
    }

    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("description", config.description)
        .add_attribute("owner", config.owner)
        .add_attribute("lockdrop_contract", config.lockdrop_contract)
        .add_attribute("oracle_contract", config.oracle_contract)
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::Name {} => query_name(deps),
        QueryMsg::Description {} => query_description(deps),
        QueryMsg::GetConfig {} => query_config(deps),
        QueryMsg::ListBonders { start_after, limit } => {
            query_list_bonders(deps, start_after, limit)
        }
        QueryMsg::BondingStatus { height, address } => {
            to_binary(&query_bonding_status(deps, env, height, address)?)
        }
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let config = CONFIG.load(deps.storage)?;

    let height = height.unwrap_or(env.block.height);

    let atom_power = get_voting_power_for_address(
        deps,
        config.lockdrop_contract.as_ref(),
        config.oracle_contract.as_ref(),
        PoolType::ATOM,
        address.clone(),
        height,
    )?;
    let usdc_power = get_voting_power_for_address(
        deps,
        config.lockdrop_contract,
        config.oracle_contract,
        PoolType::USDC,
        address,
        height,
    )?;

    let power = atom_power + usdc_power;

    Ok(VotingPowerAtHeightResponse {
        power: power.numerator().try_into()?,
        height,
    })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let config = CONFIG.load(deps.storage)?;

    let height = height.unwrap_or(env.block.height);

    let atom_power = get_voting_power_total(
        deps,
        config.lockdrop_contract.as_ref(),
        config.oracle_contract.as_ref(),
        PoolType::ATOM,
        height,
    )?;
    let usdc_power = get_voting_power_total(
        deps,
        config.lockdrop_contract,
        config.oracle_contract,
        PoolType::USDC,
        height,
    )?;

    let power = atom_power + usdc_power;

    Ok(TotalPowerAtHeightResponse {
        power: power.numerator().try_into()?,
        height,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cwd_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_name(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config.name)
}

pub fn query_description(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config.description)
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_list_bonders(
    _deps: Deps,
    _start_after: Option<String>,
    _limit: Option<u32>,
) -> StdResult<Binary> {
    // TODO: implement once the lockdrop contract is implemented.
    to_binary(&ListBondersResponse { bonders: vec![] })
}

pub fn query_bonding_status(
    _deps: Deps,
    env: Env,
    height: Option<u64>,
    _address: String,
) -> StdResult<BondingStatusResponse> {
    let height = height.unwrap_or(env.block.height);
    Ok(BondingStatusResponse {
        unbondable_abount: Uint128::zero(),
        bonding_enabled: false,
        height,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
