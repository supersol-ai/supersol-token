use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

pub struct ConcentratedLiquidityManager;

impl ConcentratedLiquidityManager {
    /// Create a new concentrated liquidity position
    pub fn create_position(
        pool: &mut SharedLiquidityPool,
        owner: Pubkey,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult<u64> {
        // Validate price range
        if lower_price >= upper_price {
            return Err(ProgramError::InvalidArgument);
        }

        // Calculate liquidity based on amounts and price range
        let liquidity =
            Self::calculate_range_liquidity(amount_a, amount_b, lower_price, upper_price)?;

        // Create new position
        let position_id = pool.next_position_id;
        let position = EnhancedLiquidityPosition {
            id: position_id,
            owner,
            ranges: vec![LiquidityRange {
                lower_price,
                upper_price,
                liquidity,
                fees_collected: 0,
                last_update: Clock::get()?.unix_timestamp,
            }],
        };

        // Update pool state
        pool.enhanced_positions.insert(position_id, position);
        pool.next_position_id = pool
            .next_position_id
            .checked_add(1)
            .ok_or(ProgramError::Overflow)?;

        Ok(position_id)
    }

    /// Add a new range to an existing position
    pub fn add_range(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check range limit
        if position.ranges.len() >= MAX_RANGES_PER_POSITION {
            return Err(ProgramError::Custom(2)); // Too many ranges
        }

        // Validate price range
        if lower_price >= upper_price {
            return Err(ProgramError::InvalidArgument);
        }

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

        Ok(())
    }

    /// Remove a range from a position
    pub fn remove_range(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        range_index: usize,
    ) -> ProgramResult<(u64, u64)> {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Validate range index
        if range_index >= position.ranges.len() {
            return Err(ProgramError::InvalidArgument);
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

        Ok((amount_a, amount_b))
    }

    /// Update position fees for a swap
    pub fn update_fees(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> ProgramResult {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Calculate fee amount
        let fee_amount = (input_amount as u128)
            .checked_mul(pool.fee as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(10000)
            .ok_or(ProgramError::Overflow)? as u64;

        // Distribute fees to active ranges based on liquidity
        let total_liquidity: u128 = position.ranges.iter().map(|r| r.liquidity as u128).sum();

        for range in &mut position.ranges {
            let range_fee = (fee_amount as u128)
                .checked_mul(range.liquidity as u128)
                .ok_or(ProgramError::Overflow)?
                .checked_div(total_liquidity)
                .ok_or(ProgramError::Overflow)? as u64;
            range.fees_collected = range
                .fees_collected
                .checked_add(range_fee)
                .ok_or(ProgramError::Overflow)?;
        }

        Ok(())
    }

    /// Collect fees from a position
    pub fn collect_fees(pool: &mut SharedLiquidityPool, position_id: u64) -> ProgramResult<u64> {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Calculate total fees
        let total_fees: u64 = position.ranges.iter().map(|r| r.fees_collected).sum();

        // Reset collected fees
        for range in &mut position.ranges {
            range.fees_collected = 0;
        }

        Ok(total_fees)
    }

    /// Calculate liquidity for a range
    fn calculate_range_liquidity(
        amount_a: u64,
        amount_b: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult<u64> {
        // Calculate liquidity based on amounts and price range
        let liquidity_a = (amount_a as u128)
            .checked_mul(upper_price as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ProgramError::Overflow)? as u128,
            )
            .ok_or(ProgramError::Overflow)? as u64;

        let liquidity_b = (amount_b as u128)
            .checked_mul(lower_price as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ProgramError::Overflow)? as u128,
            )
            .ok_or(ProgramError::Overflow)? as u64;

        Ok(liquidity_a.min(liquidity_b))
    }

    /// Calculate token amounts for a range
    fn calculate_range_token_amounts(
        liquidity: u64,
        lower_price: u64,
        upper_price: u64,
    ) -> ProgramResult<(u64, u64)> {
        let amount_a = (liquidity as u128)
            .checked_mul(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ProgramError::Overflow)? as u128,
            )
            .ok_or(ProgramError::Overflow)?
            .checked_div(upper_price as u128)
            .ok_or(ProgramError::Overflow)? as u64;

        let amount_b = (liquidity as u128)
            .checked_mul(
                upper_price
                    .checked_sub(lower_price)
                    .ok_or(ProgramError::Overflow)? as u128,
            )
            .ok_or(ProgramError::Overflow)?
            .checked_div(lower_price as u128)
            .ok_or(ProgramError::Overflow)? as u64;

        Ok((amount_a, amount_b))
    }
}
