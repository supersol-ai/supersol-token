use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

pub struct SharedLiquidityPool {
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee: u64,
    pub admin: Pubkey,
    pub price_oracle: Option<PriceOracle>,
    pub liquidity_mining: Option<LiquidityMining>,
    pub emergency_pause: Option<EmergencyPause>,
    pub positions: Vec<LiquidityPosition>,
    pub next_position_id: u64,
}

impl SharedLiquidityPool {
    /// Create a new liquidity pool
    pub fn create_pool(
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        fee: u64,
        admin: Pubkey,
    ) -> ProgramResult {
        // Validate fee is within reasonable range (0.01% to 1%)
        if fee < 1 || fee > 100 {
            return Err(ProgramError::InvalidArgument);
        }

        Ok(SharedLiquidityPool {
            token_a_mint,
            token_b_mint,
            reserve_a: 0,
            reserve_b: 0,
            fee,
            admin,
            price_oracle: None,
            liquidity_mining: None,
            emergency_pause: None,
            positions: Vec::new(),
            next_position_id: 1,
        })
    }

    /// Add liquidity to the pool
    pub fn add_liquidity(&mut self, amount_a: u64, amount_b: u64, owner: Pubkey) -> ProgramResult {
        // Check if pool is paused
        if let Some(pause) = &self.emergency_pause {
            if pause.is_paused {
                return Err(ProgramError::Custom(1)); // Pool is paused
            }
        }

        // Calculate shares based on current reserves
        let shares = if self.reserve_a == 0 && self.reserve_b == 0 {
            // First liquidity provider
            (amount_a as u128)
                .checked_mul(amount_b as u128)
                .ok_or(ProgramError::Overflow)?
                .integer_sqrt() as u64
        } else {
            // Calculate shares based on proportional contribution
            let share_a = (amount_a as u128)
                .checked_mul(self.reserve_a as u128)
                .ok_or(ProgramError::Overflow)?
                .checked_div(self.reserve_a as u128)
                .ok_or(ProgramError::Overflow)? as u64;
            let share_b = (amount_b as u128)
                .checked_mul(self.reserve_b as u128)
                .ok_or(ProgramError::Overflow)?
                .checked_div(self.reserve_b as u128)
                .ok_or(ProgramError::Overflow)? as u64;
            share_a.min(share_b)
        };

        // Update reserves
        self.reserve_a = self
            .reserve_a
            .checked_add(amount_a)
            .ok_or(ProgramError::Overflow)?;
        self.reserve_b = self
            .reserve_b
            .checked_add(amount_b)
            .ok_or(ProgramError::Overflow)?;

        // Create new position
        let position = LiquidityPosition {
            owner,
            shares,
            last_claim_time: Clock::get()?.unix_timestamp,
            accumulated_rewards: 0,
        };
        self.positions.push(position);

        Ok(())
    }

    /// Remove liquidity from the pool
    pub fn remove_liquidity(&mut self, position_id: u64, shares: u64) -> ProgramResult<(u64, u64)> {
        // Check if pool is paused
        if let Some(pause) = &self.emergency_pause {
            if pause.is_paused {
                return Err(ProgramError::Custom(1)); // Pool is paused
            }
        }

        // Find position
        let position = self
            .positions
            .iter_mut()
            .find(|p| p.id == position_id)
            .ok_or(ProgramError::InvalidAccountData)?;

        // Validate shares
        if shares > position.shares {
            return Err(ProgramError::InvalidArgument);
        }

        // Calculate amounts to return
        let amount_a = (shares as u128)
            .checked_mul(self.reserve_a as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(position.shares as u128)
            .ok_or(ProgramError::Overflow)? as u64;
        let amount_b = (shares as u128)
            .checked_mul(self.reserve_b as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(position.shares as u128)
            .ok_or(ProgramError::Overflow)? as u64;

        // Update reserves
        self.reserve_a = self
            .reserve_a
            .checked_sub(amount_a)
            .ok_or(ProgramError::Overflow)?;
        self.reserve_b = self
            .reserve_b
            .checked_sub(amount_b)
            .ok_or(ProgramError::Overflow)?;

        // Update position
        position.shares = position
            .shares
            .checked_sub(shares)
            .ok_or(ProgramError::Overflow)?;

        Ok((amount_a, amount_b))
    }

    /// Get pool price
    pub fn get_price(&self) -> ProgramResult<u64> {
        if self.reserve_b == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        (self.reserve_a as u128)
            .checked_mul(PRICE_PRECISION as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(self.reserve_b as u128)
            .ok_or(ProgramError::Overflow)
            .map(|price| price as u64)
    }

    /// Get pool TVL
    pub fn get_tvl(&self) -> ProgramResult<u64> {
        // Calculate TVL in terms of token A
        let price = self.get_price()?;
        (self.reserve_b as u128)
            .checked_mul(price as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(PRICE_PRECISION as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_add(self.reserve_a as u128)
            .ok_or(ProgramError::Overflow)
            .map(|tvl| tvl as u64)
    }
}
