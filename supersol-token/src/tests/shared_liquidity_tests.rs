use {
    crate::shared_liquidity::{SharedLiquidityChecker, SharedLiquidityCompatibility},
    crate::state::{Account, Mint},
    solana_program_option::COption,
    solana_pubkey::Pubkey,
};

#[test]
fn test_shared_liquidity_compatibility() {
    let mint = Mint {
        mint_authority: COption::None,
        supply: 1_000_000,
        decimals: 9,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let account = Account {
        mint: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        amount: 1000,
        delegate: COption::None,
        state: crate::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };

    // Test comprehensive compatibility check
    let compatibility = SharedLiquidityChecker::check_compatibility(&mint, &[account]);
    assert_eq!(compatibility, SharedLiquidityCompatibility::Compatible);
}
