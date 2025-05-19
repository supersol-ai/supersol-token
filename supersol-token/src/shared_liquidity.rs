use {
    crate::state::{Account, Mint},
    solana_program_error::{ProgramError, ProgramResult},
};

/// Maximum number of decimals allowed for shared liquidity tokens
pub const MAX_SHARED_LIQUIDITY_DECIMALS: u8 = 9;

/// Minimum supply required for shared liquidity tokens
pub const MIN_SHARED_LIQUIDITY_SUPPLY: u64 = 1_000_000; // 1M tokens minimum

/// Maximum supply allowed for shared liquidity tokens
pub const MAX_SHARED_LIQUIDITY_SUPPLY: u64 = 1_000_000_000_000; // 1T tokens maximum

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

/// Enhanced shared liquidity compatibility checks and token exchange functionality
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

    /// Calculate exchange rate between two compatible tokens
    /// Returns the amount of destination tokens for a given amount of source tokens
    pub fn calculate_exchange_rate(
        source_mint: &Mint,
        destination_mint: &Mint,
        source_amount: u64,
    ) -> Result<u64, ProgramError> {
        // Check if both tokens are compatible
        if Self::check_mint_compatibility(source_mint) != SharedLiquidityCompatibility::Compatible
            || Self::check_mint_compatibility(destination_mint)
                != SharedLiquidityCompatibility::Compatible
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Calculate exchange rate based on supply ratio
        let source_supply = source_mint.supply as f64;
        let destination_supply = destination_mint.supply as f64;
        let source_amount_f = source_amount as f64;

        // Calculate destination amount using supply ratio
        let destination_amount = (source_amount_f * destination_supply / source_supply) as u64;

        Ok(destination_amount)
    }

    /// Exchange tokens between two compatible token accounts
    pub fn exchange_tokens(
        source_account: &mut Account,
        destination_account: &mut Account,
        source_mint: &Mint,
        destination_mint: &Mint,
        amount: u64,
    ) -> Result<(), ProgramError> {
        // Check if both accounts are compatible
        if Self::check_account_compatibility(source_account)
            != SharedLiquidityCompatibility::Compatible
            || Self::check_account_compatibility(destination_account)
                != SharedLiquidityCompatibility::Compatible
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if source account has enough balance
        if source_account.amount < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Calculate destination amount
        let destination_amount =
            Self::calculate_exchange_rate(source_mint, destination_mint, amount)?;

        // Update account balances
        source_account.amount = source_account
            .amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        destination_account.amount = destination_account
            .amount
            .checked_add(destination_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_option::COption;
    use solana_pubkey::Pubkey;

    #[test]
    fn test_mint_compatibility() {
        let mut mint = Mint {
            mint_authority: COption::None,
            supply: 1_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        // Test valid mint
        assert_eq!(
            SharedLiquidityChecker::check_mint_compatibility(&mint),
            SharedLiquidityCompatibility::Compatible
        );

        // Test uninitialized mint
        mint.is_initialized = false;
        assert_eq!(
            SharedLiquidityChecker::check_mint_compatibility(&mint),
            SharedLiquidityCompatibility::NotInitialized
        );

        // Test mint with authority
        mint.is_initialized = true;
        mint.mint_authority = COption::Some(Pubkey::new_unique());
        assert_eq!(
            SharedLiquidityChecker::check_mint_compatibility(&mint),
            SharedLiquidityCompatibility::HasMintAuthority
        );

        // Test mint with too many decimals
        mint.mint_authority = COption::None;
        mint.decimals = 10;
        assert_eq!(
            SharedLiquidityChecker::check_mint_compatibility(&mint),
            SharedLiquidityCompatibility::InvalidDecimals
        );

        // Test mint with invalid supply
        mint.decimals = 9;
        mint.supply = 0;
        assert_eq!(
            SharedLiquidityChecker::check_mint_compatibility(&mint),
            SharedLiquidityCompatibility::InvalidSupply
        );
    }

    #[test]
    fn test_account_compatibility() {
        let mut account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 1000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Test valid account
        assert_eq!(
            SharedLiquidityChecker::check_account_compatibility(&account),
            SharedLiquidityCompatibility::Compatible
        );

        // Test frozen account
        account.state = crate::state::AccountState::Frozen;
        assert_eq!(
            SharedLiquidityChecker::check_account_compatibility(&account),
            SharedLiquidityCompatibility::FrozenAccounts
        );

        // Test account with delegate
        account.state = crate::state::AccountState::Initialized;
        account.delegate = COption::Some(Pubkey::new_unique());
        assert_eq!(
            SharedLiquidityChecker::check_account_compatibility(&account),
            SharedLiquidityCompatibility::HasDelegates
        );
    }

    #[test]
    fn test_token_exchange() {
        let source_mint = Mint {
            mint_authority: COption::None,
            supply: 1_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        let destination_mint = Mint {
            mint_authority: COption::None,
            supply: 2_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        let mut source_account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 1000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut destination_account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 0,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Test exchange rate calculation
        let exchange_amount =
            SharedLiquidityChecker::calculate_exchange_rate(&source_mint, &destination_mint, 1000)
                .unwrap();
        assert_eq!(exchange_amount, 2000); // 2:1 ratio based on supply

        // Test token exchange
        SharedLiquidityChecker::exchange_tokens(
            &mut source_account,
            &mut destination_account,
            &source_mint,
            &destination_mint,
            1000,
        )
        .unwrap();

        assert_eq!(source_account.amount, 0);
        assert_eq!(destination_account.amount, 2000);
    }
}
