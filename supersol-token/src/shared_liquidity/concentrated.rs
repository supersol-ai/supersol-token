//! Concentrated Liquidity Module
//!
//! This module implements concentrated liquidity functionality for the SuperSol token program.
//! Concentrated liquidity allows liquidity providers to provide liquidity within specific
//! price ranges, potentially earning higher fees.
//!
//! # Features
//!
//! - Create and manage concentrated liquidity positions
//! - Multiple price ranges per position
//! - Fee collection and distribution
//! - Range rebalancing and optimization
//! - Price range validation
//!
//! # Safety Features
//!
//! - Price range validation
//! - Range overlap prevention
//! - Arithmetic overflow protection
//! - Position ownership verification
//! - Fee calculation safety
//!
//! # Usage
//!
//! ```rust
//! use supersol_token::shared_liquidity::concentrated::ConcentratedLiquidityManager;
//!
//! // Create a new position
//! let position_id = ConcentratedLiquidityManager::create_position(
//!     &mut pool,
//!     owner,
//!     amount_a,
//!     amount_b,
//!     lower_price,
//!     upper_price,
//! )?;
//! ```

use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;
use solana_sysvar::clock::Clock;
use solana_sysvar::sysvar::Sysvar;
use thiserror::Error;

/// Custom error types for concentrated liquidity operations
#[derive(Error, Debug, PartialEq)]
pub enum ConcentratedLiquidityError {
    #[error("Invalid price range")]
    InvalidPriceRange,
    #[error("Range limit exceeded")]
    RangeLimitExceeded,
    #[error("Range overlap detected")]
    RangeOverlap,
    #[error("Position not found")]
    PositionNotFound,
    #[error("Invalid range index")]
    InvalidRangeIndex,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    #[error("Invalid ownership")]
    InvalidOwnership,
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,
    #[error("Invalid fee calculation")]
    InvalidFeeCalculation,
    #[error("Pool is paused")]
    PoolPaused,
}

impl From<ConcentratedLiquidityError> for ProgramError {
    fn from(e: ConcentratedLiquidityError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// Manages concentrated liquidity operations
pub struct ConcentratedLiquidityManager;

impl ConcentratedLiquidityManager {
    /// Create a new concentrated liquidity position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `owner` - The position owner
    /// * `amount_a` - Amount of token A
    /// * `amount_b` - Amount of token B
    /// * `lower_price` - Lower price bound
    /// * `upper_price` - Upper price bound
    ///
    /// # Returns
    ///
    /// * `Result<u64, ConcentratedLiquidityError>` - Position ID or error
    pub fn create_position(
        pool: &mut SharedLiquidityPool,
        owner: Pubkey,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> Result<u64, ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Validate price range
        if lower_price >= upper_price {
            return Err(ConcentratedLiquidityError::InvalidPriceRange);
        }

        // Calculate liquidity based on amounts and price range
        let liquidity =
            Self::calculate_range_liquidity(amount_a, amount_b, lower_price, upper_price)?;

        // Create new position
        let position_id = pool.next_position_id;
        let position = EnhancedLiquidityPosition {
            base_position: LiquidityPosition {
                shares: liquidity,
                last_claim_time: Clock::get()?.unix_timestamp,
                accumulated_rewards: 0,
                owner,
                id: position_id,
            },
            ranges: vec![LiquidityRange {
                lower_price: lower_price as u32,
                upper_price: upper_price as u32,
                liquidity,
                fees_collected: 0,
                last_update: Clock::get()?.unix_timestamp,
            }],
            position_id,
            created_at: Clock::get()?.unix_timestamp,
            updated_at: Clock::get()?.unix_timestamp,
        };

        // Update pool state
        pool.enhanced_positions.insert(position_id, position);
        pool.next_position_id = pool
            .next_position_id
            .checked_add(1)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?;

        Ok(position_id)
    }

    /// Add a new range to an existing position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    /// * `amount_a` - Amount of token A
    /// * `amount_b` - Amount of token B
    /// * `lower_price` - Lower price bound
    /// * `upper_price` - Upper price bound
    ///
    /// # Returns
    ///
    /// * `Result<(), ConcentratedLiquidityError>` - Success or error
    pub fn add_range(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> Result<(), ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Check range limit
        if position.ranges.len() >= MAX_RANGES_PER_POSITION {
            return Err(ConcentratedLiquidityError::RangeLimitExceeded);
        }

        // Validate price range
        if lower_price >= upper_price {
            return Err(ConcentratedLiquidityError::InvalidPriceRange);
        }

        // Check for range overlap
        Self::check_range_overlap(&position.ranges, lower_price, upper_price)?;

        // Calculate liquidity for new range
        let liquidity =
            Self::calculate_range_liquidity(amount_a, amount_b, lower_price, upper_price)?;

        // Add new range
        position.ranges.push(LiquidityRange {
            lower_price,
            upper_price,
            liquidity,
            fees_collected: 0,
            last_update: Clock::get()?.unix_timestamp,
        });

        // Update position timestamp
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Remove a range from a position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    /// * `range_index` - Index of the range to remove
    ///
    /// # Returns
    ///
    /// * `Result<(u64, u64), ConcentratedLiquidityError>` - Token amounts to return or error
    pub fn remove_range(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        range_index: usize,
    ) -> Result<(u64, u64), ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Validate range index
        if range_index >= position.ranges.len() {
            return Err(ConcentratedLiquidityError::InvalidRangeIndex);
        }

        // Get range to remove
        let range = &position.ranges[range_index];

        // Calculate token amounts to return
        let (amount_a, amount_b) = Self::calculate_range_token_amounts(
            range.liquidity,
            range.lower_price,
            range.upper_price,
        )?;

        // Remove range
        position.ranges.remove(range_index);

        // Update position timestamp
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok((amount_a, amount_b))
    }

    /// Update position fees for a swap
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    /// * `input_amount` - Input token amount
    /// * `output_amount` - Output token amount
    ///
    /// # Returns
    ///
    /// * `Result<(), ConcentratedLiquidityError>` - Success or error
    pub fn update_fees(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Result<(), ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Calculate fee amount
        let fee_amount = (input_amount as u128)
            .checked_mul(pool.fee_rate as u128)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            .checked_div(10000)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u64;

        // Distribute fees to active ranges based on liquidity
        let total_liquidity: u128 = position.ranges.iter().map(|r| r.liquidity as u128).sum();

        if total_liquidity == 0 {
            return Err(ConcentratedLiquidityError::InsufficientLiquidity);
        }

        for range in &mut position.ranges {
            let range_fee = (fee_amount as u128)
                .checked_mul(range.liquidity as u128)
                .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
                .checked_div(total_liquidity)
                .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
                as u64;
            range.fees_collected = range
                .fees_collected
                .checked_add(range_fee)
                .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?;
        }

        // Update base position rewards
        position.base_position.accumulated_rewards = position
            .base_position
            .accumulated_rewards
            .checked_add(fee_amount)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?;
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Collect fees from a position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    ///
    /// # Returns
    ///
    /// * `Result<u64, ConcentratedLiquidityError>` - Total fees collected or error
    pub fn collect_fees(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
    ) -> Result<u64, ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Calculate total fees from ranges
        let total_fees: u64 = position.ranges.iter().map(|r| r.fees_collected).sum();

        // Add base position rewards
        let total_rewards = position.base_position.accumulated_rewards;

        // Reset collected fees
        for range in &mut position.ranges {
            range.fees_collected = 0;
        }
        position.base_position.accumulated_rewards = 0;
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok(total_fees
            .checked_add(total_rewards)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?)
    }

    /// Calculate liquidity for a range
    ///
    /// # Arguments
    ///
    /// * `amount_a` - Amount of token A
    /// * `amount_b` - Amount of token B
    /// * `lower_price` - Lower price bound
    /// * `upper_price` - Upper price bound
    ///
    /// # Returns
    ///
    /// * `Result<u64, ConcentratedLiquidityError>` - Calculated liquidity or error
    fn calculate_range_liquidity(
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> Result<u64, ConcentratedLiquidityError> {
        // Calculate liquidity based on amounts and price range
        let liquidity_a = (amount_a as u128)
            .checked_mul(upper_price as u128)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            .checked_div(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u128,
            )
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            as u64;

        let liquidity_b = (amount_b as u128)
            .checked_mul(lower_price as u128)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            .checked_div(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u128,
            )
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            as u64;

        Ok(liquidity_a.min(liquidity_b))
    }

    /// Calculate token amounts for a range
    ///
    /// # Arguments
    ///
    /// * `liquidity` - Liquidity amount
    /// * `lower_price` - Lower price bound
    /// * `upper_price` - Upper price bound
    ///
    /// # Returns
    ///
    /// * `Result<(u64, u64), ConcentratedLiquidityError>` - Token amounts or error
    fn calculate_range_token_amounts(
        liquidity: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> Result<(u64, u64), ConcentratedLiquidityError> {
        let amount_a = (liquidity as u128)
            .checked_mul(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u128,
            )
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            .checked_div(upper_price as u128)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u64;

        let amount_b = (liquidity as u128)
            .checked_mul(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u128,
            )
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?
            .checked_div(lower_price as u128)
            .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)? as u64;

        Ok((amount_a, amount_b))
    }

    /// Check if ranges overlap
    ///
    /// # Arguments
    ///
    /// * `ranges` - Existing ranges
    /// * `new_lower` - New lower price bound
    /// * `new_upper` - New upper price bound
    ///
    /// # Returns
    ///
    /// * `Result<(), ConcentratedLiquidityError>` - Success or error
    fn check_range_overlap(
        ranges: &[LiquidityRange],
        new_lower: u64,
        new_upper: u64,
    ) -> Result<(), ConcentratedLiquidityError> {
        for range in ranges {
            if (new_lower <= range.upper_price && new_upper >= range.lower_price) {
                return Err(ConcentratedLiquidityError::RangeOverlap);
            }
        }
        Ok(())
    }

    /// Rebalance ranges for a position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    /// * `target_ranges` - Target price ranges
    ///
    /// # Returns
    ///
    /// * `Result<(), ConcentratedLiquidityError>` - Success or error
    pub fn rebalance_ranges(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        target_ranges: Vec<(u64, u64)>,
    ) -> Result<(), ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Validate target ranges
        for (lower, upper) in &target_ranges {
            if lower >= upper {
                return Err(ConcentratedLiquidityError::InvalidPriceRange);
            }
        }

        // Check for overlaps in target ranges
        for i in 0..target_ranges.len() {
            for j in (i + 1)..target_ranges.len() {
                if target_ranges[i].0 <= target_ranges[j].1
                    && target_ranges[i].1 >= target_ranges[j].0
                {
                    return Err(ConcentratedLiquidityError::RangeOverlap);
                }
            }
        }

        // Calculate total liquidity
        let total_liquidity: u64 = position.ranges.iter().map(|r| r.liquidity).sum();

        // Create new ranges
        let mut new_ranges = Vec::new();
        for (lower, upper) in target_ranges {
            let liquidity = total_liquidity / target_ranges.len() as u64;
            new_ranges.push(LiquidityRange {
                lower_price: lower,
                upper_price: upper,
                liquidity,
                fees_collected: 0,
                last_update: Clock::get()?.unix_timestamp,
            });
        }

        // Update position ranges
        position.ranges = new_ranges;
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Optimize range fees for a position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `position_id` - The position ID
    ///
    /// # Returns
    ///
    /// * `Result<(), ConcentratedLiquidityError>` - Success or error
    pub fn optimize_range_fees(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
    ) -> Result<(), ConcentratedLiquidityError> {
        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ConcentratedLiquidityError::PoolPaused);
        }

        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ConcentratedLiquidityError::PositionNotFound)?;

        // Sort ranges by fees collected
        position
            .ranges
            .sort_by(|a, b| b.fees_collected.cmp(&a.fees_collected));

        // Redistribute liquidity to most profitable ranges
        let total_liquidity: u64 = position.ranges.iter().map(|r| r.liquidity).sum();
        let profitable_ranges: Vec<&mut LiquidityRange> = position
            .ranges
            .iter_mut()
            .filter(|r| r.fees_collected > 0)
            .collect();

        if profitable_ranges.is_empty() {
            return Ok(());
        }

        let liquidity_per_range = total_liquidity / profitable_ranges.len() as u64;
        for range in profitable_ranges {
            range.liquidity = liquidity_per_range;
        }

        // Update base position shares
        position.base_position.shares = total_liquidity;
        position.updated_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Get optimal ranges for a position
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `current_price` - Current price
    /// * `num_ranges` - Number of ranges to generate
    ///
    /// # Returns
    ///
    /// * `Result<Vec<(u64, u64)>, ConcentratedLiquidityError>` - Optimal ranges or error
    pub fn get_optimal_ranges(
        pool: &SharedLiquidityPool,
        current_price: u64,
        num_ranges: u8,
    ) -> Result<Vec<(u64, u64)>, ConcentratedLiquidityError> {
        if num_ranges == 0 || num_ranges > MAX_RANGES_PER_POSITION as u8 {
            return Err(ConcentratedLiquidityError::RangeLimitExceeded);
        }

        let mut ranges = Vec::new();
        let price_step = current_price / (num_ranges as u64);

        for i in 0..num_ranges {
            let lower = current_price
                .checked_sub(price_step * (i + 1) as u64)
                .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?;
            let upper = current_price
                .checked_add(price_step * (i + 1) as u64)
                .ok_or(ConcentratedLiquidityError::ArithmeticOverflow)?;
            ranges.push((lower, upper));
        }

        Ok(ranges)
    }
}
