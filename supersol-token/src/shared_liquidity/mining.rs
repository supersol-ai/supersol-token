use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;
use solana_sysvar::clock::Clock;
use solana_sysvar::sysvar::Sysvar;
use thiserror::Error;

/// Custom error types for liquidity mining operations
#[derive(Error, Debug, PartialEq)]
pub enum MiningError {
    #[error("Invalid reward rate")]
    InvalidRewardRate,
    #[error("Invalid staking period")]
    InvalidStakingPeriod,
    #[error("Staking period not ended")]
    StakingPeriodNotEnded,
    #[error("Insufficient stake")]
    InsufficientStake,
    #[error("Stake not found")]
    StakeNotFound,
    #[error("Reward calculation failed")]
    RewardCalculationFailed,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

impl From<MiningError> for ProgramError {
    fn from(e: MiningError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub struct LiquidityMiningManager;

impl LiquidityMiningManager {
    /// Initialize liquidity mining for a pool
    pub fn initialize_mining(
        pool: &mut SharedLiquidityPool,
        reward_token: Pubkey,
        reward_rate: u64,
        staking_period: i64,
    ) -> ProgramResult {
        // Validate parameters
        if reward_rate == 0 {
            return Err(MiningError::InvalidRewardRate.into());
        }

        if staking_period <= 0 {
            return Err(MiningError::InvalidStakingPeriod.into());
        }

        // Create mining config
        let mining = LiquidityMining {
            reward_token,
            reward_rate,
            staking_period,
            total_staked: 0,
            last_reward_update: Clock::get()?.unix_timestamp,
            accumulated_rewards: 0,
            staking_accounts: Vec::new(),
        };

        pool.liquidity_mining = mining;
        Ok(())
    }

    /// Stake liquidity tokens
    pub fn stake_tokens(
        pool: &mut SharedLiquidityPool,
        staker: &Pubkey,
        amount: u64,
    ) -> ProgramResult {
        // Validate stake amount
        if amount == 0 {
            return Err(MiningError::InsufficientStake.into());
        }

        // Update total staked amount
        pool.liquidity_mining.total_staked = pool
            .liquidity_mining
            .total_staked
            .checked_add(amount)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        // Create or update staking account
        let current_time = Clock::get()?.unix_timestamp;
        let staking_account = StakingAccount {
            staker: *staker,
            amount,
            start_time: current_time,
            last_claim_time: current_time,
            accumulated_rewards: 0,
        };

        // Update or add staking account
        if let Some(existing) = pool
            .liquidity_mining
            .staking_accounts
            .iter_mut()
            .find(|acc| acc.staker == *staker)
        {
            existing.amount = existing
                .amount
                .checked_add(amount)
                .ok_or(MiningError::ArithmeticOverflow.into())?;
            existing.start_time = current_time;
        } else {
            pool.liquidity_mining.staking_accounts.push(staking_account);
        }

        Ok(())
    }

    /// Unstake liquidity tokens
    pub fn unstake_tokens(
        pool: &mut SharedLiquidityPool,
        staker: &Pubkey,
        amount: u64,
    ) -> ProgramResult {
        // Find staking account
        let staking_account = pool
            .liquidity_mining
            .staking_accounts
            .iter_mut()
            .find(|acc| acc.staker == *staker)
            .ok_or(MiningError::StakeNotFound)?;

        // Validate unstake amount
        if amount > staking_account.amount {
            return Err(MiningError::InsufficientStake.into());
        }

        // Check staking period
        let current_time = Clock::get()?.unix_timestamp;
        if current_time
            .checked_sub(staking_account.start_time)
            .ok_or(MiningError::ArithmeticOverflow.into())?
            < pool.liquidity_mining.staking_period
        {
            return Err(MiningError::StakingPeriodNotEnded.into());
        }

        // Update staking account
        staking_account.amount = staking_account
            .amount
            .checked_sub(amount)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        // Update total staked amount
        pool.liquidity_mining.total_staked = pool
            .liquidity_mining
            .total_staked
            .checked_sub(amount)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        // Remove staking account if fully unstaked
        if staking_account.amount == 0 {
            pool.liquidity_mining
                .staking_accounts
                .retain(|acc| acc.staker != *staker);
        }

        Ok(())
    }

    /// Calculate and claim rewards
    pub fn claim_rewards(pool: &mut SharedLiquidityPool, staker: &Pubkey) -> ProgramResult<u64> {
        // Find staking account
        let staking_account = pool
            .liquidity_mining
            .staking_accounts
            .iter_mut()
            .find(|acc| acc.staker == *staker)
            .ok_or(MiningError::StakeNotFound)?;

        // Calculate time elapsed since last claim
        let current_time = Clock::get()?.unix_timestamp;
        let time_elapsed = current_time
            .checked_sub(staking_account.last_claim_time)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        // Calculate rewards
        let rewards = if pool.liquidity_mining.total_staked > 0 {
            let reward_per_second = pool.liquidity_mining.reward_rate as f64
                / pool.liquidity_mining.staking_period as f64;
            let staker_share =
                staking_account.amount as f64 / pool.liquidity_mining.total_staked as f64;
            (reward_per_second * time_elapsed as f64 * staker_share) as u64
        } else {
            0
        };

        // Update staking account
        staking_account.last_claim_time = current_time;
        staking_account.accumulated_rewards = staking_account
            .accumulated_rewards
            .checked_add(rewards)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        Ok(rewards)
    }

    /// Get staking info for a user
    pub fn get_staking_info(
        pool: &SharedLiquidityPool,
        staker: &Pubkey,
    ) -> ProgramResult<StakingInfo> {
        // Find staking account
        let staking_account = pool
            .liquidity_mining
            .staking_accounts
            .iter()
            .find(|acc| acc.staker == *staker)
            .ok_or(MiningError::StakeNotFound)?;

        // Calculate current rewards
        let current_time = Clock::get()?.unix_timestamp;
        let time_elapsed = current_time
            .checked_sub(staking_account.last_claim_time)
            .ok_or(MiningError::ArithmeticOverflow.into())?;

        let current_rewards = if pool.liquidity_mining.total_staked > 0 {
            let reward_per_second = pool.liquidity_mining.reward_rate as f64
                / pool.liquidity_mining.staking_period as f64;
            let staker_share =
                staking_account.amount as f64 / pool.liquidity_mining.total_staked as f64;
            (reward_per_second * time_elapsed as f64 * staker_share) as u64
        } else {
            0
        };

        Ok(StakingInfo {
            staked_amount: staking_account.amount,
            start_time: staking_account.start_time,
            last_claim_time: staking_account.last_claim_time,
            accumulated_rewards: staking_account.accumulated_rewards,
            current_rewards,
            total_staked: pool.liquidity_mining.total_staked,
            reward_rate: pool.liquidity_mining.reward_rate,
            staking_period: pool.liquidity_mining.staking_period,
        })
    }
}
