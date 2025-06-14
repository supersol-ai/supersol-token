use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;
use solana_sysvar::clock::Clock;
use solana_sysvar::sysvar::Sysvar;
use thiserror::Error;

/// Custom error types for price oracle operations
#[derive(Error, Debug, PartialEq)]
pub enum OracleError {
    #[error("Invalid update interval")]
    InvalidUpdateInterval,
    #[error("Invalid price deviation")]
    InvalidPriceDeviation,
    #[error("Oracle not active")]
    OracleNotActive,
    #[error("Update too soon")]
    UpdateTooSoon,
    #[error("Price deviation too high")]
    PriceDeviationTooHigh,
    #[error("Price is stale")]
    PriceStale,
    #[error("Invalid price")]
    InvalidPrice,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

impl From<OracleError> for ProgramError {
    fn from(e: OracleError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub struct PriceOracleManager;

impl PriceOracleManager {
    /// Initialize price oracle for a pool
    pub fn initialize_oracle(
        pool: &mut SharedLiquidityPool,
        min_update_interval: i64,
        max_price_deviation: u16,
    ) -> ProgramResult {
        // Validate parameters
        if min_update_interval <= 0 {
            return Err(OracleError::InvalidUpdateInterval.into());
        }

        if max_price_deviation == 0 || max_price_deviation > 10000 {
            return Err(OracleError::InvalidPriceDeviation.into());
        }

        // Create oracle config
        let oracle = PriceOracle {
            last_price: 0.0,
            last_update: Clock::get()?.unix_timestamp,
            min_update_interval,
            max_price_deviation,
            is_active: true,
        };

        pool.price_oracle = oracle;
        Ok(())
    }

    /// Update oracle price
    pub fn update_price(pool: &mut SharedLiquidityPool, new_price: f64) -> ProgramResult {
        // Check if oracle is active
        if !pool.price_oracle.is_active {
            return Err(OracleError::OracleNotActive.into());
        }

        // Validate price
        if new_price <= 0.0 {
            return Err(OracleError::InvalidPrice.into());
        }

        // Get current time
        let current_time = Clock::get()?.unix_timestamp;

        // Check update interval
        if current_time
            .checked_sub(pool.price_oracle.last_update)
            .ok_or(OracleError::ArithmeticOverflow.into())?
            < pool.price_oracle.min_update_interval
        {
            return Err(OracleError::UpdateTooSoon.into());
        }

        // Check price deviation
        if pool.price_oracle.last_price > 0.0 {
            let price_diff = if new_price > pool.price_oracle.last_price {
                new_price - pool.price_oracle.last_price
            } else {
                pool.price_oracle.last_price - new_price
            };

            let deviation = (price_diff / pool.price_oracle.last_price) * 10000.0;

            if deviation > pool.price_oracle.max_price_deviation as f64 {
                return Err(OracleError::PriceDeviationTooHigh.into());
            }
        }

        // Update oracle state
        pool.price_oracle.last_price = new_price;
        pool.price_oracle.last_update = current_time;

        Ok(())
    }

    /// Get oracle price
    pub fn get_price(pool: &SharedLiquidityPool) -> ProgramResult<f64> {
        // Check if oracle is active
        if !pool.price_oracle.is_active {
            return Err(OracleError::OracleNotActive.into());
        }

        // Check if price is stale
        let current_time = Clock::get()?.unix_timestamp;
        if current_time
            .checked_sub(pool.price_oracle.last_update)
            .ok_or(OracleError::ArithmeticOverflow.into())?
            > pool
                .price_oracle
                .min_update_interval
                .checked_mul(2)
                .ok_or(OracleError::ArithmeticOverflow.into())?
        {
            return Err(OracleError::PriceStale.into());
        }

        Ok(pool.price_oracle.last_price)
    }

    /// Pause oracle
    pub fn pause_oracle(pool: &mut SharedLiquidityPool) -> ProgramResult {
        pool.price_oracle.is_active = false;
        Ok(())
    }

    /// Resume oracle
    pub fn resume_oracle(pool: &mut SharedLiquidityPool) -> ProgramResult {
        pool.price_oracle.is_active = true;
        Ok(())
    }

    /// Calculate price impact for a swap
    pub fn calculate_price_impact(
        pool: &SharedLiquidityPool,
        input_amount: u64,
        output_amount: u64,
        is_token_a_to_b: bool,
    ) -> ProgramResult<f64> {
        // Get current price
        let current_price = Self::get_price(pool)?;

        // Calculate execution price
        let execution_price = if is_token_a_to_b {
            output_amount as f64 / input_amount as f64
        } else {
            input_amount as f64 / output_amount as f64
        };

        // Calculate price impact
        let price_impact = if is_token_a_to_b {
            (current_price - execution_price) / current_price
        } else {
            (execution_price - current_price) / current_price
        };

        Ok(price_impact.abs())
    }

    /// Check if price is within acceptable range
    pub fn is_price_valid(pool: &SharedLiquidityPool, price: f64) -> ProgramResult<bool> {
        // Get current price
        let current_price = Self::get_price(pool)?;

        // Calculate deviation
        let deviation = if price > current_price {
            (price - current_price) / current_price
        } else {
            (current_price - price) / current_price
        };

        Ok(deviation <= pool.price_oracle.max_price_deviation as f64 / 10000.0)
    }
}
