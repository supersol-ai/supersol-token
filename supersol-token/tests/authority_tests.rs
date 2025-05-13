use {
    crate::{
        instruction::{set_authority, AuthorityType},
        state::{Account, Mint},
    },
    solana_program::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
    spl_token::instruction::{burn, mint_to},
    test_utils::*,
};

#[test]
fn test_mint_authority_updates() {
    let program_id = spl_token::id();
    let owner_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let mut mint_account = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);
    let mut rent_sysvar = rent_sysvar();

    // Initialize mint with owner as mint authority
    do_process_instruction(
        initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
        vec![&mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(mint.mint_authority.is_some());
    assert_eq!(mint.mint_authority.unwrap(), owner_key);

    // Update mint authority to new owner
    let new_owner_key = Pubkey::new_unique();
    do_process_instruction(
        set_authority(
            &program_id,
            &mint_key,
            Some(&new_owner_key),
            AuthorityType::MintTokens,
            &owner_key,
            &[],
        )
        .unwrap(),
        vec![&mut mint_account],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(mint.mint_authority.is_some());
    assert_eq!(mint.mint_authority.unwrap(), new_owner_key);

    // Disable mint authority (make supply fixed)
    do_process_instruction(
        set_authority(
            &program_id,
            &mint_key,
            None,
            AuthorityType::MintTokens,
            &new_owner_key,
            &[],
        )
        .unwrap(),
        vec![&mut mint_account],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(mint.mint_authority.is_none());
}

#[test]
fn test_burn_authority_updates() {
    let program_id = spl_token::id();
    let owner_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let mut mint_account = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);
    let mut rent_sysvar = rent_sysvar();

    // Initialize mint and account
    do_process_instruction(
        initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
        vec![&mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    let account_key = Pubkey::new_unique();
    let mut account = SolanaAccount::new(account_minimum_balance(), Account::LEN, &program_id);

    do_process_instruction(
        initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
        vec![&mut account, &mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    // Mint some tokens
    do_process_instruction(
        mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
        vec![&mut mint_account, &mut account],
        &[Check::success()],
    )
    .unwrap();

    // Update account owner (burn authority)
    let new_owner_key = Pubkey::new_unique();
    do_process_instruction(
        set_authority(
            &program_id,
            &account_key,
            Some(&new_owner_key),
            AuthorityType::AccountOwner,
            &owner_key,
            &[],
        )
        .unwrap(),
        vec![&mut account],
        &[Check::success()],
    )
    .unwrap();

    let account = Account::unpack(&account.data).unwrap();
    assert_eq!(account.owner, new_owner_key);
    assert!(account.delegate.is_none()); // Delegate should be cleared

    // Verify old owner can't burn
    let result = do_process_instruction(
        burn(&program_id, &account_key, &mint_key, &owner_key, &[], 100).unwrap(),
        vec![&mut account, &mut mint_account],
        &[Check::success()],
    );
    assert_eq!(result, Err(ProgramError::InvalidAccountData));

    // Verify new owner can burn
    do_process_instruction(
        burn(
            &program_id,
            &account_key,
            &mint_key,
            &new_owner_key,
            &[],
            100,
        )
        .unwrap(),
        vec![&mut account, &mut mint_account],
        &[Check::success()],
    )
    .unwrap();

    let account = Account::unpack(&account.data).unwrap();
    assert_eq!(account.amount, 900); // 1000 - 100
}
