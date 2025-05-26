use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

pub struct LiquidityMiningManager;

impl LiquidityMiningManager {
    /// Initialize liquidity mining for a pool
    pub fn initialize_mining(
        pool: &mut SharedLiquidityPool,
        total_rewards: u64,
        start_time: i64,
        end_time: i64,
        reward_rate: u64,
    ) -> ProgramResult {
        // Validate parameters
        if start_time >= end_time {
            return Err(ProgramError::InvalidArgument);
        }

        if reward_rate == 0 {
            return Err(ProgramError::InvalidArgument);
        }

        // Create mining config
        let mining = LiquidityMining {
            total_rewards,
            remaining_rewards: total_rewards,
            start_time,
            end_time,
            reward_rate,
            total_shares: 0,
            is_active: true,
        };

        pool.liquidity_mining = Some(mining);
        Ok(())
    }

    /// Update mining rewards for a position
    pub fn update_rewards(pool: &mut SharedLiquidityPool, position_id: u64) -> ProgramResult {
        // Get mining config
        let mining = pool
            .liquidity_mining
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check if mining is active
        if !mining.is_active {
            return Ok(());
        }

        // Get current time
        let current_time = Clock::get()?.unix_timestamp;

        // Check if mining period has started
        if current_time < mining.start_time {
            return Ok(());
        }

        // Check if mining period has ended
        if current_time >= mining.end_time {
            mining.is_active = false;
            return Ok(());
        }

        // Find position
        let position = pool
            .positions
            .iter_mut()
            .find(|p| p.id == position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Calculate time elapsed since last claim
        let time_elapsed = current_time
            .checked_sub(position.last_claim_time)
            .ok_or(ProgramError::Overflow)?;

        // Calculate rewards
        let rewards = if mining.total_shares > 0 {
            (position.shares as u128)
                .checked_mul(mining.reward_rate as u128)
                .ok_or(ProgramError::Overflow)?
                .checked_mul(time_elapsed as u128)
                .ok_or(ProgramError::Overflow)?
                .checked_div(mining.total_shares as u128)
                .ok_or(ProgramError::Overflow)? as u64
        } else {
            0
        };

        // Update position rewards
        position.accumulated_rewards = position
            .accumulated_rewards
            .checked_add(rewards)
            .ok_or(ProgramError::Overflow)?;
        position.last_claim_time = current_time;

        // Update remaining rewards
        mining.remaining_rewards = mining
            .remaining_rewards
            .checked_sub(rewards)
            .ok_or(ProgramError::Overflow)?;

        Ok(())
    }

    /// Claim mining rewards for a position
    pub fn claim_rewards(pool: &mut SharedLiquidityPool, position_id: u64) -> ProgramResult<u64> {
        // Update rewards first
        Self::update_rewards(pool, position_id)?;

        // Find position
        let position = pool
            .positions
            .iter_mut()
            .find(|p| p.id == position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Get accumulated rewards
        let rewards = position.accumulated_rewards;

        // Reset accumulated rewards
        position.accumulated_rewards = 0;

        Ok(rewards)
    }

    /// Add shares to mining
    pub fn add_shares(pool: &mut SharedLiquidityPool, shares: u64) -> ProgramResult {
        // Get mining config
        let mining = pool
            .liquidity_mining
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Update total shares
        mining.total_shares = mining
            .total_shares
            .checked_add(shares)
            .ok_or(ProgramError::Overflow)?;

        Ok(())
    }

    /// Remove shares from mining
    pub fn remove_shares(pool: &mut SharedLiquidityPool, shares: u64) -> ProgramResult {
        // Get mining config
        let mining = pool
            .liquidity_mining
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Update total shares
        mining.total_shares = mining
            .total_shares
            .checked_sub(shares)
            .ok_or(ProgramError::Overflow)?;

        Ok(())
    }
}
