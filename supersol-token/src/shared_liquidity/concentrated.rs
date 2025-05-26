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

    /// Check if ranges overlap
    fn check_range_overlap(
        ranges: &[LiquidityRange],
        new_lower: u64,
        new_upper: u64,
    ) -> ProgramResult {
        for range in ranges {
            if (new_lower <= range.upper_price && new_upper >= range.lower_price) {
                return Err(ProgramError::Custom(10)); // Ranges overlap
            }
        }
        Ok(())
    }

    /// Rebalance ranges to optimize fee collection
    pub fn rebalance_ranges(
        pool: &mut SharedLiquidityPool,
        position_id: u64,
        target_ranges: Vec<(u64, u64)>,
    ) -> ProgramResult {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Validate target ranges
        for (lower, upper) in &target_ranges {
            if lower >= upper {
                return Err(ProgramError::InvalidArgument);
            }
            Self::check_range_overlap(&position.ranges, *lower, *upper)?;
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

        Ok(())
    }

    /// Optimize range fees by merging adjacent ranges
    pub fn optimize_range_fees(pool: &mut SharedLiquidityPool, position_id: u64) -> ProgramResult {
        // Find position
        let position = pool
            .enhanced_positions
            .get_mut(&position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Sort ranges by lower price
        position
            .ranges
            .sort_by(|a, b| a.lower_price.cmp(&b.lower_price));

        // Merge adjacent ranges with similar fees
        let mut merged_ranges = Vec::new();
        let mut current_range = position.ranges[0].clone();

        for range in position.ranges.iter().skip(1) {
            if range.lower_price == current_range.upper_price {
                // Merge ranges
                current_range.upper_price = range.upper_price;
                current_range.liquidity = current_range
                    .liquidity
                    .checked_add(range.liquidity)
                    .ok_or(ProgramError::Overflow)?;
                current_range.fees_collected = current_range
                    .fees_collected
                    .checked_add(range.fees_collected)
                    .ok_or(ProgramError::Overflow)?;
            } else {
                merged_ranges.push(current_range.clone());
                current_range = range.clone();
            }
        }
        merged_ranges.push(current_range);

        // Update position ranges
        position.ranges = merged_ranges;

        Ok(())
    }

    /// Get optimal range distribution based on current price
    pub fn get_optimal_ranges(
        pool: &SharedLiquidityPool,
        current_price: u64,
        num_ranges: u8,
    ) -> ProgramResult<Vec<(u64, u64)>> {
        if num_ranges > MAX_RANGES_PER_POSITION {
            return Err(ProgramError::InvalidArgument);
        }

        let mut ranges = Vec::new();
        let price_step = (MAX_RANGE_WIDTH - MIN_RANGE_WIDTH) / num_ranges as u32;

        for i in 0..num_ranges {
            let lower = current_price
                .checked_mul(10000 - (price_step * (i + 1)))
                .ok_or(ProgramError::Overflow)?
                .checked_div(10000)
                .ok_or(ProgramError::Overflow)?;
            let upper = current_price
                .checked_mul(10000 + (price_step * (i + 1)))
                .ok_or(ProgramError::Overflow)?
                .checked_div(10000)
                .ok_or(ProgramError::Overflow)?;
            ranges.push((lower, upper));
        }

        Ok(ranges)
    }
}
