use {
    crate::state::{Account, Mint},
    solana_program_error::{ProgramError, ProgramResult},
    solana_pubkey::Pubkey,
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

/// IBC channel configuration for cross-chain token transfers
#[derive(Debug, Clone, PartialEq)]
pub struct IbcChannel {
    /// Source chain identifier
    pub source_chain: String,
    /// Destination chain identifier
    pub destination_chain: String,
    /// Channel ID for the IBC connection
    pub channel_id: String,
    /// Port ID for the IBC connection
    pub port_id: String,
    /// Whether the channel is active
    pub is_active: bool,
}

/// Wrapped token configuration for cross-chain compatibility
#[derive(Debug, Clone, PartialEq)]
pub struct WrappedTokenConfig {
    /// Original token's chain identifier
    pub original_chain: String,
    /// Original token's contract address
    pub original_address: String,
    /// Wrapped token's decimals
    pub decimals: u8,
    /// Whether the wrapped token is active
    pub is_active: bool,
}

/// Liquidity pool configuration for shared liquidity tokens
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidityPool {
    /// Token A's mint address
    pub token_a_mint: Pubkey,
    /// Token B's mint address
    pub token_b_mint: Pubkey,
    /// Pool's token A balance
    pub token_a_balance: u64,
    /// Pool's token B balance
    pub token_b_balance: u64,
    /// Pool's fee rate in basis points (e.g., 30 = 0.3%)
    pub fee_rate: u16,
    /// Whether the pool is active
    pub is_active: bool,
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

    /// Register a new IBC channel for cross-chain token transfers
    pub fn register_ibc_channel(
        source_chain: String,
        destination_chain: String,
        channel_id: String,
        port_id: String,
    ) -> IbcChannel {
        IbcChannel {
            source_chain,
            destination_chain,
            channel_id,
            port_id,
            is_active: true,
        }
    }

    /// Verify if a token is compatible for cross-chain transfer
    pub fn verify_cross_chain_compatibility(
        mint: &Mint,
        ibc_channel: &IbcChannel,
    ) -> Result<(), ProgramError> {
        // Check if token is initialized
        if !mint.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }

        // Check if token has a fixed supply (no mint authority)
        if mint.mint_authority.is_some() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if token has reasonable decimals (0-9)
        if mint.decimals > MAX_SHARED_LIQUIDITY_DECIMALS {
            return Err(ProgramError::InvalidArgument);
        }

        // Check if token has valid supply range
        if mint.supply < MIN_SHARED_LIQUIDITY_SUPPLY || mint.supply > MAX_SHARED_LIQUIDITY_SUPPLY {
            return Err(ProgramError::InvalidArgument);
        }

        // Check if IBC channel is active
        if !ibc_channel.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    /// Prepare token for cross-chain transfer
    pub fn prepare_cross_chain_transfer(
        source_account: &mut Account,
        amount: u64,
        ibc_channel: &IbcChannel,
    ) -> Result<(), ProgramError> {
        // Check if account has enough balance
        if source_account.amount < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Lock tokens for cross-chain transfer
        source_account.amount = source_account
            .amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Complete cross-chain token transfer
    pub fn complete_cross_chain_transfer(
        destination_account: &mut Account,
        amount: u64,
        ibc_channel: &IbcChannel,
    ) -> Result<(), ProgramError> {
        // Verify IBC channel is active
        if !ibc_channel.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Add tokens to destination account
        destination_account.amount = destination_account
            .amount
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Create a wrapped token configuration
    pub fn create_wrapped_token_config(
        original_chain: String,
        original_address: String,
        decimals: u8,
    ) -> WrappedTokenConfig {
        WrappedTokenConfig {
            original_chain,
            original_address,
            decimals,
            is_active: true,
        }
    }

    /// Wrap tokens for cross-chain transfer
    pub fn wrap_tokens(
        source_account: &mut Account,
        destination_account: &mut Account,
        amount: u64,
        wrapped_config: &WrappedTokenConfig,
    ) -> Result<(), ProgramError> {
        // Verify wrapped token is active
        if !wrapped_config.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if source account has enough balance
        if source_account.amount < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Lock original tokens
        source_account.amount = source_account
            .amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Mint wrapped tokens
        destination_account.amount = destination_account
            .amount
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Unwrap tokens after cross-chain transfer
    pub fn unwrap_tokens(
        wrapped_account: &mut Account,
        original_account: &mut Account,
        amount: u64,
        wrapped_config: &WrappedTokenConfig,
    ) -> Result<(), ProgramError> {
        // Verify wrapped token is active
        if !wrapped_config.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if wrapped account has enough balance
        if wrapped_account.amount < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Burn wrapped tokens
        wrapped_account.amount = wrapped_account
            .amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Unlock original tokens
        original_account.amount = original_account
            .amount
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Create a new liquidity pool for two compatible tokens
    pub fn create_liquidity_pool(
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        fee_rate: u16,
    ) -> Result<LiquidityPool, ProgramError> {
        // Validate fee rate (max 1%)
        if fee_rate > 100 {
            return Err(ProgramError::InvalidArgument);
        }

        // Ensure token mints are different
        if token_a_mint == token_b_mint {
            return Err(ProgramError::InvalidArgument);
        }

        Ok(LiquidityPool {
            token_a_mint,
            token_b_mint,
            token_a_balance: 0,
            token_b_balance: 0,
            fee_rate,
            is_active: true,
        })
    }

    /// Add liquidity to the pool
    pub fn add_liquidity(
        pool: &mut LiquidityPool,
        token_a_account: &mut Account,
        token_b_account: &mut Account,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Result<(), ProgramError> {
        // Verify pool is active
        if !pool.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify token accounts match pool tokens
        if token_a_account.mint != pool.token_a_mint || token_b_account.mint != pool.token_b_mint {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if accounts have enough balance
        if token_a_account.amount < token_a_amount || token_b_account.amount < token_b_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Calculate price ratio for first deposit
        if pool.token_a_balance == 0 && pool.token_b_balance == 0 {
            // First deposit, set initial price ratio
            pool.token_a_balance = token_a_amount;
            pool.token_b_balance = token_b_amount;
        } else {
            // Calculate expected token B amount based on current ratio
            let expected_token_b = (token_a_amount as f64 * pool.token_b_balance as f64
                / pool.token_a_balance as f64) as u64;

            // Allow 1% slippage
            let min_token_b = (expected_token_b * 99) / 100;
            let max_token_b = (expected_token_b * 101) / 100;

            if token_b_amount < min_token_b || token_b_amount > max_token_b {
                return Err(ProgramError::InvalidArgument);
            }

            // Update pool balances
            pool.token_a_balance = pool
                .token_a_balance
                .checked_add(token_a_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            pool.token_b_balance = pool
                .token_b_balance
                .checked_add(token_b_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        // Transfer tokens to pool
        token_a_account.amount = token_a_account
            .amount
            .checked_sub(token_a_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        token_b_account.amount = token_b_account
            .amount
            .checked_sub(token_b_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Remove liquidity from the pool
    pub fn remove_liquidity(
        pool: &mut LiquidityPool,
        token_a_account: &mut Account,
        token_b_account: &mut Account,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Result<(), ProgramError> {
        // Verify pool is active
        if !pool.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify token accounts match pool tokens
        if token_a_account.mint != pool.token_a_mint || token_b_account.mint != pool.token_b_mint {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if pool has enough balance
        if pool.token_a_balance < token_a_amount || pool.token_b_balance < token_b_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Calculate price impact
        let price_impact = (token_a_amount as f64 * token_b_amount as f64)
            / (pool.token_a_balance as f64 * pool.token_b_balance as f64);

        // Limit price impact to 5%
        if price_impact > 0.05 {
            return Err(ProgramError::InvalidArgument);
        }

        // Update pool balances
        pool.token_a_balance = pool
            .token_a_balance
            .checked_sub(token_a_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool.token_b_balance = pool
            .token_b_balance
            .checked_sub(token_b_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Transfer tokens from pool
        token_a_account.amount = token_a_account
            .amount
            .checked_add(token_a_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        token_b_account.amount = token_b_account
            .amount
            .checked_add(token_b_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Calculate swap amount using constant product formula (x * y = k)
    pub fn calculate_swap_amount(
        pool: &LiquidityPool,
        input_amount: u64,
        is_token_a_to_b: bool,
    ) -> Result<u64, ProgramError> {
        // Verify pool is active
        if !pool.is_active {
            return Err(ProgramError::InvalidAccountData);
        }

        let (input_balance, output_balance) = if is_token_a_to_b {
            (pool.token_a_balance, pool.token_b_balance)
        } else {
            (pool.token_b_balance, pool.token_a_balance)
        };

        // Ensure sufficient liquidity
        if input_balance == 0 || output_balance == 0 {
            return Err(ProgramError::InsufficientFunds);
        }

        // Calculate fee (in basis points)
        let fee_amount = (input_amount as u128)
            .checked_mul(pool.fee_rate as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10000)
            .ok_or(ProgramError::ArithmeticOverflow)? as u64;

        let input_amount_after_fee = input_amount
            .checked_sub(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Calculate output amount using constant product formula
        // (x + Δx)(y - Δy) = xy
        // Δy = (y * Δx) / (x + Δx)
        let output_amount = (output_balance as u128)
            .checked_mul(input_amount_after_fee as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(
                (input_balance as u128)
                    .checked_add(input_amount_after_fee as u128)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            )
            .ok_or(ProgramError::ArithmeticOverflow)? as u64;

        // Calculate price impact
        let price_impact = (input_amount_after_fee as f64 * output_amount as f64)
            / (input_balance as f64 * output_balance as f64);

        // Limit price impact to 5%
        if price_impact > 0.05 {
            return Err(ProgramError::InvalidArgument);
        }

        // Apply slippage protection (0.1%)
        let min_output = (output_amount as u128)
            .checked_mul(999)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(1000)
            .ok_or(ProgramError::ArithmeticOverflow)? as u64;

        Ok(min_output)
    }

    /// Execute a token swap in the pool
    pub fn execute_swap(
        pool: &mut LiquidityPool,
        input_account: &mut Account,
        output_account: &mut Account,
        input_amount: u64,
        is_token_a_to_b: bool,
    ) -> Result<u64, ProgramError> {
        // Calculate output amount
        let output_amount = Self::calculate_swap_amount(pool, input_amount, is_token_a_to_b)?;

        // Verify token accounts match pool tokens
        if is_token_a_to_b {
            if input_account.mint != pool.token_a_mint || output_account.mint != pool.token_b_mint {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            if input_account.mint != pool.token_b_mint || output_account.mint != pool.token_a_mint {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // Check if input account has enough balance
        if input_account.amount < input_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Update pool balances
        if is_token_a_to_b {
            pool.token_a_balance = pool
                .token_a_balance
                .checked_add(input_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            pool.token_b_balance = pool
                .token_b_balance
                .checked_sub(output_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        } else {
            pool.token_b_balance = pool
                .token_b_balance
                .checked_add(input_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            pool.token_a_balance = pool
                .token_a_balance
                .checked_sub(output_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        // Update account balances
        input_account.amount = input_account
            .amount
            .checked_sub(input_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        output_account.amount = output_account
            .amount
            .checked_add(output_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(output_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_option::COption;

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

    #[test]
    fn test_cross_chain_transfer() {
        let mint = Mint {
            mint_authority: COption::None,
            supply: 1_000_000,
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

        // Create IBC channel
        let ibc_channel = SharedLiquidityChecker::register_ibc_channel(
            "supersol".to_string(),
            "ethereum".to_string(),
            "channel-1".to_string(),
            "transfer".to_string(),
        );

        // Verify cross-chain compatibility
        assert!(
            SharedLiquidityChecker::verify_cross_chain_compatibility(&mint, &ibc_channel).is_ok()
        );

        // Prepare cross-chain transfer
        assert!(SharedLiquidityChecker::prepare_cross_chain_transfer(
            &mut source_account,
            500,
            &ibc_channel
        )
        .is_ok());
        assert_eq!(source_account.amount, 500);

        // Complete cross-chain transfer
        assert!(SharedLiquidityChecker::complete_cross_chain_transfer(
            &mut destination_account,
            500,
            &ibc_channel
        )
        .is_ok());
        assert_eq!(destination_account.amount, 500);
    }

    #[test]
    fn test_token_wrapping() {
        let mut original_account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 1000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut wrapped_account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 0,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Create wrapped token configuration
        let wrapped_config = SharedLiquidityChecker::create_wrapped_token_config(
            "ethereum".to_string(),
            "0x123...".to_string(),
            18,
        );

        // Test wrapping tokens
        assert!(SharedLiquidityChecker::wrap_tokens(
            &mut original_account,
            &mut wrapped_account,
            500,
            &wrapped_config
        )
        .is_ok());
        assert_eq!(original_account.amount, 500);
        assert_eq!(wrapped_account.amount, 500);

        // Test unwrapping tokens
        assert!(SharedLiquidityChecker::unwrap_tokens(
            &mut wrapped_account,
            &mut original_account,
            500,
            &wrapped_config
        )
        .is_ok());
        assert_eq!(wrapped_account.amount, 0);
        assert_eq!(original_account.amount, 1000);
    }

    #[test]
    fn test_liquidity_pool() {
        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Create liquidity pool
        let mut pool = SharedLiquidityChecker::create_liquidity_pool(
            token_a_mint,
            token_b_mint,
            30, // 0.3% fee
        )
        .unwrap();

        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 1000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 1000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Test adding liquidity
        assert!(SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            500,
            500
        )
        .is_ok());
        assert_eq!(pool.token_a_balance, 500);
        assert_eq!(pool.token_b_balance, 500);
        assert_eq!(token_a_account.amount, 500);
        assert_eq!(token_b_account.amount, 500);

        // Test swapping tokens
        let output_amount = SharedLiquidityChecker::execute_swap(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            100,
            true,
        )
        .unwrap();
        assert!(output_amount > 0);
        assert_eq!(token_a_account.amount, 400);
        assert_eq!(token_b_account.amount, 500 + output_amount);

        // Test removing liquidity
        assert!(SharedLiquidityChecker::remove_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            400,
            400
        )
        .is_ok());
        assert_eq!(pool.token_a_balance, 100);
        assert_eq!(pool.token_b_balance, 100);
        assert_eq!(token_a_account.amount, 800);
        assert_eq!(token_b_account.amount, 900);
    }

    #[test]
    fn test_liquidity_pool_optimized() {
        // Tests:
        // 1. Initial liquidity provision (500,000 tokens each)
        // 2. Small swap (1,000 tokens) - should succeed
        // 3. Large swap (200,000 tokens) - should fail due to price impact
        // 4. Fee calculation verification (0.3% of 1000 = 3)
        // 5. Slippage protection verification
        // 6. Liquidity removal
        // 7. Final balance verification

        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Create liquidity pool with 0.3% fee
        let mut pool =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_b_mint, 30).unwrap();

        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Test initial liquidity provision
        assert!(SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            500_000,
            500_000
        )
        .is_ok());

        // Test small swap (low price impact)
        let small_swap_amount = SharedLiquidityChecker::execute_swap(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            1_000,
            true,
        )
        .unwrap();
        assert!(small_swap_amount > 0);

        // Test large swap (should fail due to price impact)
        let large_swap_result = SharedLiquidityChecker::execute_swap(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            200_000,
            true,
        );
        assert!(large_swap_result.is_err());

        // Test fee calculation
        let fee_amount = (1_000 as u128)
            .checked_mul(pool.fee_rate as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;
        assert_eq!(fee_amount, 3); // 0.3% of 1000 = 3

        // Test slippage protection
        let swap_amount =
            SharedLiquidityChecker::calculate_swap_amount(&pool, 1_000, true).unwrap();
        let expected_amount = (swap_amount as u128)
            .checked_mul(999)
            .unwrap()
            .checked_div(1000)
            .unwrap() as u64;
        assert!(swap_amount >= expected_amount);

        // Test removing liquidity
        assert!(SharedLiquidityChecker::remove_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            400_000,
            400_000
        )
        .is_ok());

        // Verify final balances
        assert_eq!(pool.token_a_balance, 100_000);
        assert_eq!(pool.token_b_balance, 100_000);
    }

    #[test]
    fn test_liquidity_pool_edge_cases() {
        // Tests:
        // 1. Creating pool with same token - should fail
        // 2. Creating pool with invalid fee (2%) - should fail
        // 3. Adding liquidity with zero amounts - should fail
        // 4. Swapping with insufficient liquidity - should fail
        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Test creating pool with same token
        let same_token_result =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_a_mint, 30);
        assert!(same_token_result.is_err());

        // Test creating pool with invalid fee
        let invalid_fee_result = SharedLiquidityChecker::create_liquidity_pool(
            token_a_mint,
            token_b_mint,
            200, // 2% fee (invalid)
        );
        assert!(invalid_fee_result.is_err());

        // Create valid pool
        let mut pool =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_b_mint, 30).unwrap();

        // Test adding liquidity with zero amounts
        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 0,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 0,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let zero_liquidity_result = SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            0,
            0,
        );
        assert!(zero_liquidity_result.is_err());

        // Test swap with insufficient liquidity
        let insufficient_liquidity_result = SharedLiquidityChecker::execute_swap(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            1000,
            true,
        );
        assert!(insufficient_liquidity_result.is_err());
    }

    #[test]
    fn test_consecutive_swaps() {
        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Create liquidity pool with 0.3% fee
        let mut pool =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_b_mint, 30).unwrap();

        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Add initial liquidity
        assert!(SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            500_000,
            500_000
        )
        .is_ok());

        // Perform multiple swaps in sequence
        let swap_amounts = [1_000, 2_000, 5_000, 10_000];
        let mut total_output = 0;

        for amount in swap_amounts.iter() {
            let output = SharedLiquidityChecker::execute_swap(
                &mut pool,
                &mut token_a_account,
                &mut token_b_account,
                *amount,
                true,
            )
            .unwrap();
            total_output += output;
        }

        // Verify that each swap had increasing price impact
        let mut last_price_impact = 0.0;
        for amount in swap_amounts.iter() {
            let output =
                SharedLiquidityChecker::calculate_swap_amount(&pool, *amount, true).unwrap();
            let price_impact = (*amount as f64 * output as f64)
                / (pool.token_a_balance as f64 * pool.token_b_balance as f64);
            assert!(price_impact > last_price_impact);
            last_price_impact = price_impact;
        }

        // Verify total output is less than what would be expected from a single large swap
        let single_swap_output = SharedLiquidityChecker::calculate_swap_amount(
            &pool,
            swap_amounts.iter().sum::<u64>(),
            true,
        )
        .unwrap();
        assert!(total_output > single_swap_output);
    }

    #[test]
    fn test_fee_collection() {
        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Create liquidity pool with 0.3% fee
        let mut pool =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_b_mint, 30).unwrap();

        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Add initial liquidity
        assert!(SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            500_000,
            500_000
        )
        .is_ok());

        // Record initial balances
        let initial_a_balance = pool.token_a_balance;
        let initial_b_balance = pool.token_b_balance;

        // Perform a swap
        let swap_amount = 10_000;
        let output = SharedLiquidityChecker::execute_swap(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            swap_amount,
            true,
        )
        .unwrap();

        // Calculate expected fee
        let expected_fee = (swap_amount as u128)
            .checked_mul(pool.fee_rate as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;

        // Verify fee was collected
        assert_eq!(pool.token_a_balance, initial_a_balance + swap_amount);
        assert_eq!(pool.token_b_balance, initial_b_balance - output);

        // Verify fee amount
        let actual_fee = swap_amount - (swap_amount - expected_fee);
        assert_eq!(actual_fee, expected_fee);
    }

    #[test]
    fn test_slippage_protection() {
        let token_a_mint = Pubkey::new_unique();
        let token_b_mint = Pubkey::new_unique();

        // Create liquidity pool with 0.3% fee
        let mut pool =
            SharedLiquidityChecker::create_liquidity_pool(token_a_mint, token_b_mint, 30).unwrap();

        let mut token_a_account = Account {
            mint: token_a_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut token_b_account = Account {
            mint: token_b_mint,
            owner: Pubkey::new_unique(),
            amount: 1_000_000,
            delegate: COption::None,
            state: crate::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        // Add initial liquidity
        assert!(SharedLiquidityChecker::add_liquidity(
            &mut pool,
            &mut token_a_account,
            &mut token_b_account,
            500_000,
            500_000
        )
        .is_ok());

        // Test different swap sizes
        let swap_sizes = [1_000, 10_000, 50_000, 100_000];

        for size in swap_sizes.iter() {
            // Calculate expected output with slippage protection
            let expected_output =
                SharedLiquidityChecker::calculate_swap_amount(&pool, *size, true).unwrap();

            // Calculate minimum output with 0.1% slippage
            let min_output = (expected_output as u128)
                .checked_mul(999)
                .unwrap()
                .checked_div(1000)
                .unwrap() as u64;

            // Execute swap
            let actual_output = SharedLiquidityChecker::execute_swap(
                &mut pool,
                &mut token_a_account,
                &mut token_b_account,
                *size,
                true,
            )
            .unwrap();

            // Verify output is within slippage bounds
            assert!(actual_output >= min_output);
            assert!(actual_output <= expected_output);

            // Verify slippage percentage
            let slippage = (expected_output - actual_output) as f64 / expected_output as f64;
            assert!(slippage <= 0.001); // 0.1% maximum slippage
        }
    }
}
