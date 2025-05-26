use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

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
            return Err(ProgramError::InvalidArgument);
        }

        // Create pause config
        let pause = EmergencyPause {
            is_paused: false,
            pause_timestamp: 0,
            max_pause_duration,
            admin_pubkey,
        };

        pool.emergency_pause = Some(pause);
        Ok(())
    }

    /// Pause pool operations
    pub fn pause_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        // Get pause config
        let pause = pool
            .emergency_pause
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check admin authorization
        if pause.admin_pubkey != admin_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if already paused
        if pause.is_paused {
            return Err(ProgramError::Custom(7)); // Already paused
        }

        // Update pause state
        pause.is_paused = true;
        pause.pause_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }

    /// Resume pool operations
    pub fn resume_pool(pool: &mut SharedLiquidityPool, admin_pubkey: Pubkey) -> ProgramResult {
        // Get pause config
        let pause = pool
            .emergency_pause
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Check admin authorization
        if pause.admin_pubkey != admin_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if already resumed
        if !pause.is_paused {
            return Err(ProgramError::Custom(8)); // Already resumed
        }

        // Update pause state
        pause.is_paused = false;
        pause.pause_timestamp = 0;

        Ok(())
    }

    /// Check if pool is paused
    pub fn is_paused(pool: &SharedLiquidityPool) -> ProgramResult<bool> {
        // Get pause config
        let pause = pool
            .emergency_pause
            .as_ref()
            .ok_or(ProgramError::InvalidAccountData)?;

        // If not paused, return false
        if !pause.is_paused {
            return Ok(false);
        }

        // Check pause duration
        let current_time = Clock::get()?.unix_timestamp;
        let pause_duration = current_time
            .checked_sub(pause.pause_timestamp)
            .ok_or(ProgramError::Overflow)?;

        // If pause duration exceeded, return false
        if pause_duration > pause.max_pause_duration {
            return Ok(false);
        }

        Ok(true)
    }

    /// Get remaining pause time
    pub fn get_remaining_pause_time(pool: &SharedLiquidityPool) -> ProgramResult<i64> {
        // Get pause config
        let pause = pool
            .emergency_pause
            .as_ref()
            .ok_or(ProgramError::InvalidAccountData)?;

        // If not paused, return 0
        if !pause.is_paused {
            return Ok(0);
        }

        // Calculate remaining time
        let current_time = Clock::get()?.unix_timestamp;
        let pause_duration = current_time
            .checked_sub(pause.pause_timestamp)
            .ok_or(ProgramError::Overflow)?;

        let remaining_time = pause
            .max_pause_duration
            .checked_sub(pause_duration)
            .ok_or(ProgramError::Overflow)?;

        Ok(remaining_time)
    }
}
