mod compatibility;
mod concentrated;
mod constants;
mod emergency;
mod flash_loan;
mod mining;
mod oracle;
mod pool;
mod swap;
mod types;

pub use compatibility::*;
pub use concentrated::*;
pub use constants::*;
pub use emergency::*;
pub use flash_loan::*;
pub use mining::*;
pub use oracle::*;
pub use pool::*;
pub use swap::*;
pub use types::*;

use solana_program_error::{ProgramError, ProgramResult};

/// Main struct for shared liquidity operations
pub struct SharedLiquidityChecker;

impl SharedLiquidityChecker {
    /// Create a new liquidity pool
    pub fn create_liquidity_pool(
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        fee: u64,
        admin: Pubkey,
    ) -> ProgramResult<SharedLiquidityPool> {
        SharedLiquidityPool::create_pool(token_a_mint, token_b_mint, fee, admin)
    }

    /// Add liquidity to a pool
    pub fn add_liquidity(
        pool: &mut SharedLiquidityPool,
        amount_a: u64,
        amount_b: u64,
        owner: Pubkey,
    ) -> ProgramResult {
        pool.add_liquidity(amount_a, amount_b, owner)
    }

    /// Remove liquidity from a pool
    pub fn remove_liquidity(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        shares: u64,
    ) -> ProgramResult<(u64, u64)> {
        pool.remove_liquidity(position_id, shares)
    }

    /// Create a concentrated liquidity position
    pub fn create_concentrated_position(
        pool: &mut SharedLiquidityPool,
        owner: Pubkey,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult<u64> {
        ConcentratedLiquidityManager::create_position(
            pool,
            owner,
            amount_a,
            amount_b,
            lower_price,
            upper_price,
        )
    }

    /// Add a range to a concentrated position
    pub fn add_range_to_position(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult {
        ConcentratedLiquidityManager::add_range(
            pool,
            position_id,
            amount_a,
            amount_b,
            lower_price,
            upper_price,
        )
    }

    /// Remove a range from a concentrated position
    pub fn remove_range_from_position(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        range_index: usize,
    ) -> ProgramResult<(u64, u64)> {
        ConcentratedLiquidityManager::remove_range(pool, position_id, range_index)
    }

    /// Initialize liquidity mining
    pub fn initialize_mining(
        pool: &mut SharedLiquidityPool,
        total_rewards: u64,
        start_time: i64,
        end_time: i64,
        reward_rate: u64,
    ) -> ProgramResult {
        LiquidityMiningManager::initialize_mining(
            pool,
            total_rewards,
            start_time,
            end_time,
            reward_rate,
        )
    }

    /// Claim mining rewards
    pub fn claim_mining_rewards(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
    ) -> ProgramResult<u64> {
        LiquidityMiningManager::claim_rewards(pool, position_id)
    }

    /// Initialize price oracle
    pub fn initialize_oracle(
        pool: &mut SharedLiquidityPool,
        min_update_interval: i64,
        max_price_deviation: u64,
    ) -> ProgramResult {
        PriceOracleManager::initialize_oracle(pool, min_update_interval, max_price_deviation)
    }

    /// Update oracle price
    pub fn update_oracle_price(pool: &mut SharedLiquidityPool, new_price: u64) -> ProgramResult {
        PriceOracleManager::update_price(pool, new_price)
    }

    /// Initialize emergency pause
    pub fn initialize_emergency_pause(
        pool: &mut SharedLiquidityPool,
        max_pause_duration: i64,
        admin_pubkey: Pubkey,
    ) -> ProgramResult {
        EmergencyManager::initialize_pause(pool, max_pause_duration, admin_pubkey)
    }

    /// Pause pool operations
    pub fn pause_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        EmergencyManager::pause_pool(pool, admin_pubkey)
    }

    /// Resume pool operations
    pub fn resume_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        EmergencyManager::resume_pool(pool, admin_pubkey)
    }

    /// Execute a swap
    pub fn execute_swap(
        pool: &mut SharedLiquidityPool,
        input_amount: u64,
        min_output_amount: u64,
        is_token_a_to_b: bool,
    ) -> ProgramResult<u64> {
        SwapManager::execute_swap(pool, input_amount, min_output_amount, is_token_a_to_b)
    }

    /// Find best swap path
    pub fn find_best_swap_path(
        pools: &[SharedLiquidityPool],
        input_token: Pubkey,
        output_token: Pubkey,
        input_amount: u64,
        min_output_amount: u64,
    ) -> ProgramResult<Option<SwapPath>> {
        SwapManager::find_best_swap_path(
            pools,
            input_token,
            output_token,
            input_amount,
            min_output_amount,
        )
    }

    /// Execute a multi-hop swap
    pub fn execute_multi_hop_swap(
        pools: &mut [SharedLiquidityPool],
        path: &SwapPath,
        input_amount: u64,
        min_output_amount: u64,
    ) -> ProgramResult<u64> {
        SwapManager::execute_multi_hop_swap(pools, path, input_amount, min_output_amount)
    }
}
