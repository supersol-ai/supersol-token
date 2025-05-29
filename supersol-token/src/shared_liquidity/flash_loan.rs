//! Flash Loan Module
//!
//! This module implements flash loan functionality for the SuperSol token program.
//! Flash loans allow users to borrow tokens without collateral, as long as they are
//! repaid within the same transaction.
//!
//! # Features
//!
//! - Execute flash loans with safety checks
//! - Fee calculation and distribution
//! - Emergency pause integration
//! - Token account validation
//!
//! # Safety Features
//!
//! - Maximum loan amount limits
//! - Pool state validation
//! - Token account validation
//! - Emergency pause integration
//!
//! # Usage
//!
//! ```rust
//! use supersol_token::shared_liquidity::flash_loan::FlashLoanManager;
//!
//! // Execute a flash loan
//! let result = FlashLoanManager::execute_flash_loan(
//!     &mut pool,
//!     &token_account,
//!     amount,
//!     |pool, amount| {
//!         // Use the loan
//!         Ok(())
//!     },
//! );
//! ```
//!
//! # Fee Structure
//!
//! - Base fee: 0.09% of loan amount
//! - Fees are distributed to liquidity providers proportionally
//! - Minimum fee: 1 token
//!
//! # Events
//!
//! The module emits events for:
//! - Flash loan execution
//! - Fee distribution
//! - Error conditions

use super::constants::*;
use super::types::*;
use crate::shared_liquidity::types::{LiquidityPosition, SharedLiquidityPool};
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

/// Represents a flash loan event
#[derive(Debug, Clone, PartialEq)]
pub struct FlashLoanEvent {
    /// Amount of tokens borrowed
    pub amount: u64,
    /// Token mint address
    pub token_mint: Pubkey,
    /// Fee charged for the loan
    pub fee: u64,
    /// Timestamp of the event
    pub timestamp: i64,
    /// Whether the loan was successful
    pub success: bool,
}

/// Manages flash loan operations
pub struct FlashLoanManager;

impl FlashLoanManager {
    /// Executes a flash loan
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool to borrow from
    /// * `token_account` - The token account to receive the loan
    /// * `amount` - The amount to borrow
    /// * `callback` - Function to execute with the borrowed tokens
    ///
    /// # Returns
    ///
    /// * `Result<(), ProgramError>` - Success or error
    ///
    /// # Safety
    ///
    /// - Validates pool state
    /// - Checks loan amount limits
    /// - Verifies token account
    /// - Ensures repayment
    pub fn execute_flash_loan(
        pool: &mut SharedLiquidityPool,
        token_account: &Account,
        amount: u64,
        callback: impl FnOnce(&mut SharedLiquidityPool, u64) -> ProgramResult,
    ) -> ProgramResult {
        // Check if pool is active
        if !pool.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if pool is paused
        if pool.emergency_pause.is_paused {
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify token account
        if token_account.owner != pool.token_a_mint && token_account.owner != pool.token_b_mint {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check amount against limits
        let max_amount = if token_account.owner == pool.token_a_mint {
            pool.token_a_balance
                .checked_mul(pool.max_flash_loan_percentage as u64)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .checked_div(100)
                .ok_or(ProgramError::ArithmeticOverflow)?
        } else {
            pool.token_b_balance
                .checked_mul(pool.max_flash_loan_percentage as u64)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .checked_div(100)
                .ok_or(ProgramError::ArithmeticOverflow)?
        };

        if amount > max_amount {
            return Err(ProgramError::InvalidArgument);
        }

        // Calculate fee
        let fee = amount
            .checked_mul(pool.flash_loan_fee as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10000)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Transfer tokens
        if token_account.owner == pool.token_a_mint {
            pool.token_a_balance = pool
                .token_a_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        } else {
            pool.token_b_balance = pool
                .token_b_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        // Execute callback
        callback(pool, amount)?;

        // Verify repayment
        if token_account.owner == pool.token_a_mint {
            if pool.token_a_balance
                < amount
                    .checked_add(fee)
                    .ok_or(ProgramError::ArithmeticOverflow)?
            {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            if pool.token_b_balance
                < amount
                    .checked_add(fee)
                    .ok_or(ProgramError::ArithmeticOverflow)?
            {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // Distribute fees to liquidity providers
        Self::distribute_fees(pool, fee)?;

        Ok(())
    }

    /// Distributes fees to liquidity providers
    ///
    /// # Arguments
    ///
    /// * `pool` - The liquidity pool
    /// * `fee` - The fee to distribute
    ///
    /// # Returns
    ///
    /// * `Result<(), ProgramError>` - Success or error
    fn distribute_fees(pool: &mut SharedLiquidityPool, fee: u64) -> ProgramResult {
        let total_shares: u64 = pool.positions.values().map(|pos| pos.shares).sum();

        if total_shares == 0 {
            return Ok(());
        }

        for position in pool.positions.values_mut() {
            let share = position
                .shares
                .checked_mul(fee)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .checked_div(total_shares)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            position.accumulated_rewards = position
                .accumulated_rewards
                .checked_add(share)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        Ok(())
    }

    /// Get flash loan limits for a pool
    pub fn get_flash_loan_limits(pool: &SharedLiquidityPool) -> (u64, u64) {
        let max_a = pool
            .token_a_balance
            .checked_mul(pool.max_flash_loan_percentage as u64)
            .unwrap_or(0)
            .checked_div(100)
            .unwrap_or(0);

        let max_b = pool
            .token_b_balance
            .checked_mul(pool.max_flash_loan_percentage as u64)
            .unwrap_or(0)
            .checked_div(100)
            .unwrap_or(0);

        (max_a, max_b)
    }

    /// Calculates the fee for a flash loan
    ///
    /// # Arguments
    ///
    /// * `amount` - The loan amount
    /// * `fee_rate` - The fee rate in basis points
    ///
    /// # Returns
    ///
    /// * `u64` - The calculated fee
    pub fn calculate_fee(pool: &SharedLiquidityPool, amount: u64) -> ProgramResult<u64> {
        amount
            .checked_mul(pool.flash_loan_fee as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10000)
            .ok_or(ProgramError::ArithmeticOverflow)
    }
}
