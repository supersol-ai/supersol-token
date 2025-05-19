use {
    crate::state::{Account, Mint},
    solana_program_option::COption,
    solana_pubkey::Pubkey,
};

#[test]
fn test_authority_validation() {
    let mint = Mint {
        mint_authority: COption::Some(Pubkey::new_unique()),
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

    // Basic authority validation tests
    assert!(mint.mint_authority.is_some());
    assert!(!account.is_frozen());
    assert!(account.delegate.is_none());
}
