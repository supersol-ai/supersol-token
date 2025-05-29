use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;
use solana_sysvar::clock::Clock;
use solana_sysvar::sysvar::Sysvar;
use thiserror::Error;

/// Custom error types for emergency operations
#[derive(Error, Debug, PartialEq)]
pub enum EmergencyError {
    #[error("Invalid pause duration")]
    InvalidPauseDuration,
    #[error("Unauthorized admin")]
    UnauthorizedAdmin,
    #[error("Pool already paused")]
    AlreadyPaused,
    #[error("Pool already resumed")]
    AlreadyResumed,
    #[error("Invalid account data")]
    InvalidAccountData,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

impl From<EmergencyError> for ProgramError {
    fn from(e: EmergencyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub struct EmergencyManager;

impl EmergencyManager {
    /// Initialize emergency pause for a pool
    pub fn initialize_pause(
        pool: &mut SharedLiquidityPool,
        max_pause_duration: i64,
        admin_pubkey: Pubkey,
    ) -> ProgramResult {
        // Validate parameters
        if max_pause_duration <= 0 {
            return Err(EmergencyError::InvalidPauseDuration.into());
        }

        // Create pause config
        let pause = EmergencyPause {
            is_paused: false,
            pause_timestamp: 0,
            max_pause_duration,
            admin: admin_pubkey,
        };

        pool.emergency_pause = pause;
        Ok(())
    }

    /// Pause pool operations
    pub fn pause_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        // Check admin authorization
        if pool.emergency_pause.admin != admin_pubkey {
            return Err(EmergencyError::UnauthorizedAdmin.into());
        }

        // Check if already paused
        if pool.emergency_pause.is_paused {
            return Err(EmergencyError::AlreadyPaused.into());
        }

        // Update pause state
        pool.emergency_pause.is_paused = true;
        pool.emergency_pause.pause_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Resume pool operations
    pub fn resume_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        // Check admin authorization
        if pool.emergency_pause.admin != admin_pubkey {
            return Err(EmergencyError::UnauthorizedAdmin.into());
        }

        // Check if already resumed
        if !pool.emergency_pause.is_paused {
            return Err(EmergencyError::AlreadyResumed.into());
        }

        // Update pause state
        pool.emergency_pause.is_paused = false;
        pool.emergency_pause.pause_timestamp = 0;

        Ok(())
    }

    /// Check if pool is paused
    pub fn is_paused(pool: &SharedLiquidityPool) -> ProgramResult<bool> {
        // If not paused, return false
        if !pool.emergency_pause.is_paused {
            return Ok(false);
        }

        // Check pause duration
        let current_time = Clock::get()?.unix_timestamp;
        let pause_duration = current_time
            .checked_sub(pool.emergency_pause.pause_timestamp)
            .ok_or(EmergencyError::ArithmeticOverflow)?;

        // If pause duration exceeded, return false
        if pause_duration > pool.emergency_pause.max_pause_duration {
            return Ok(false);
        }

        Ok(true)
    }

    /// Get remaining pause time
    pub fn get_remaining_pause_time(pool: &SharedLiquidityPool) -> ProgramResult<i64> {
        // If not paused, return 0
        if !pool.emergency_pause.is_paused {
            return Ok(0);
        }

        // Calculate remaining time
        let current_time = Clock::get()?.unix_timestamp;
        let pause_duration = current_time
            .checked_sub(pool.emergency_pause.pause_timestamp)
            .ok_or(EmergencyError::ArithmeticOverflow)?;

        let remaining_time = pool
            .emergency_pause
            .max_pause_duration
            .checked_sub(pause_duration)
            .ok_or(EmergencyError::ArithmeticOverflow)?;

        Ok(remaining_time)
    }
}
