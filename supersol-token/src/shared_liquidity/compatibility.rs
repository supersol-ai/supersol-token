use super::constants::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};

/// Shared liquidity compatibility check result
#[derive(Debug, PartialEq)]
pub enum SharedLiquidityCompatibility {
    /// Token meets all shared liquidity requirements
    Compatible,
    /// Token is not initialized
    NotInitialized,
    /// Token has a mint authority (not fixed supply)
    HasMintAuthority,
    /// Token has too many decimals
    InvalidDecimals,
    /// Token supply is outside allowed range
    InvalidSupply,
    /// Token has frozen accounts
    FrozenAccounts,
    /// Token has delegated accounts
    HasDelegates,
}

impl SharedLiquidityCompatibility {
    /// Convert compatibility result to ProgramResult
    pub fn to_program_result(&self) -> ProgramResult {
        match self {
            Self::Compatible => Ok(()),
            Self::NotInitialized => Err(ProgramError::UninitializedAccount),
            Self::HasMintAuthority => Err(ProgramError::InvalidAccountData),
            Self::InvalidDecimals => Err(ProgramError::InvalidArgument),
            Self::InvalidSupply => Err(ProgramError::InvalidArgument),
            Self::FrozenAccounts => Err(ProgramError::InvalidAccountData),
            Self::HasDelegates => Err(ProgramError::InvalidAccountData),
        }
    }
}

pub struct SharedLiquidityChecker;

impl SharedLiquidityChecker {
    /// Check if a mint is compatible with shared liquidity
    pub fn check_mint_compatibility(mint: &Mint) -> SharedLiquidityCompatibility {
        // Check if token is initialized
        if !mint.is_initialized {
            return SharedLiquidityCompatibility::NotInitialized;
        }

        // Check if token has a fixed supply (no mint authority)
        if mint.mint_authority.is_some() {
            return SharedLiquidityCompatibility::HasMintAuthority;
        }

        // Check if token has reasonable decimals (0-9)
        if mint.decimals > MAX_SHARED_LIQUIDITY_DECIMALS {
            return SharedLiquidityCompatibility::InvalidDecimals;
        }

        // Check if token has valid supply range
        if mint.supply < MIN_SHARED_LIQUIDITY_SUPPLY || mint.supply > MAX_SHARED_LIQUIDITY_SUPPLY {
            return SharedLiquidityCompatibility::InvalidSupply;
        }

        SharedLiquidityCompatibility::Compatible
    }

    /// Check if an account is compatible with shared liquidity
    pub fn check_account_compatibility(account: &Account) -> SharedLiquidityCompatibility {
        // Check if account is frozen
        if account.is_frozen() {
            return SharedLiquidityCompatibility::FrozenAccounts;
        }

        // Check if account has any delegates
        if account.delegate.is_some() {
            return SharedLiquidityCompatibility::HasDelegates;
        }

        SharedLiquidityCompatibility::Compatible
    }

    /// Comprehensive check for shared liquidity compatibility
    pub fn check_compatibility(mint: &Mint, accounts: &[Account]) -> SharedLiquidityCompatibility {
        // First check mint compatibility
        let mint_compatibility = Self::check_mint_compatibility(mint);
        if mint_compatibility != SharedLiquidityCompatibility::Compatible {
            return mint_compatibility;
        }

        // Then check all accounts
        for account in accounts {
            let account_compatibility = Self::check_account_compatibility(account);
            if account_compatibility != SharedLiquidityCompatibility::Compatible {
                return account_compatibility;
            }
        }

        SharedLiquidityCompatibility::Compatible
    }
}
