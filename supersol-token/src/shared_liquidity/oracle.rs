use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

pub struct PriceOracleManager;

impl PriceOracleManager {
    /// Initialize price oracle for a pool
    pub fn initialize_oracle(
        pool: &mut SharedLiquidityPool,
        min_update_interval: i64,
        max_price_deviation: u64,
    ) -> ProgramResult {
        // Validate parameters
        if min_update_interval <= 0 {
            return Err(ProgramError::InvalidArgument);
        }

        if max_price_deviation == 0 || max_price_deviation > 10000 {
            return Err(ProgramError::InvalidArgument);
        }

        // Create oracle config
        let oracle = PriceOracle {
            last_observed_price: 0,
            last_update: 0,
            min_update_interval,
            max_price_deviation,
            is_active: true,
        };

        pool.price_oracle = Some(oracle);
        Ok(())
    }

    /// Update oracle price
    pub fn update_price(pool: &mut SharedLiquidityPool, new_price: u64) -> ProgramResult {
        // Get oracle config
        let oracle = pool
            .price_oracle
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check if oracle is active
        if !oracle.is_active {
            return Err(ProgramError::Custom(3)); // Oracle is not active
        }

        // Get current time
        let current_time = Clock::get()?.unix_timestamp;

        // Check update interval
        if current_time
            .checked_sub(oracle.last_update)
            .ok_or(ProgramError::Overflow)?
            < oracle.min_update_interval
        {
            return Err(ProgramError::Custom(4)); // Too soon to update
        }

        // Check price deviation
        if oracle.last_observed_price > 0 {
            let price_diff = if new_price > oracle.last_observed_price {
                new_price
                    .checked_sub(oracle.last_observed_price)
                    .ok_or(ProgramError::Overflow)?
            } else {
                oracle
                    .last_observed_price
                    .checked_sub(new_price)
                    .ok_or(ProgramError::Overflow)?
            };

            let deviation = (price_diff as u128)
                .checked_mul(10000)
                .ok_or(ProgramError::Overflow)?
                .checked_div(oracle.last_observed_price as u128)
                .ok_or(ProgramError::Overflow)? as u64;

            if deviation > oracle.max_price_deviation {
                return Err(ProgramError::Custom(5)); // Price deviation too high
            }
        }

        // Update oracle state
        oracle.last_observed_price = new_price;
        oracle.last_update = current_time;

        Ok(())
    }

    /// Get oracle price
    pub fn get_price(pool: &SharedLiquidityPool) -> ProgramResult<u64> {
        // Get oracle config
        let oracle = pool
            .price_oracle
            .as_ref()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check if oracle is active
        if !oracle.is_active {
            return Err(ProgramError::Custom(3)); // Oracle is not active
        }

        // Check if price is stale
        let current_time = Clock::get()?.unix_timestamp;
        if current_time
            .checked_sub(oracle.last_update)
            .ok_or(ProgramError::Overflow)?
            > oracle
                .min_update_interval
                .checked_mul(2)
                .ok_or(ProgramError::Overflow)?
        {
            return Err(ProgramError::Custom(6)); // Price is stale
        }

        Ok(oracle.last_observed_price)
    }

    /// Pause oracle
    pub fn pause_oracle(pool: &mut SharedLiquidityPool) -> ProgramResult {
        // Get oracle config
        let oracle = pool
            .price_oracle
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        oracle.is_active = false;
        Ok(())
    }

    /// Resume oracle
    pub fn resume_oracle(pool: &mut SharedLiquidityPool) -> ProgramResult {
        // Get oracle config
        let oracle = pool
            .price_oracle
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        oracle.is_active = true;
        Ok(())
    }
}
