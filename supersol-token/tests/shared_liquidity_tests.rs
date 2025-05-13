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
fn test_shared_liquidity_eligibility() {
    let program_id = spl_token::id();
    let owner_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let mut mint_account = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);
    let mut rent_sysvar = rent_sysvar();

    // Initialize mint with mint authority
    do_process_instruction(
        initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
        vec![&mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(!mint.is_eligible_for_shared_liquidity()); // Not eligible due to mint authority

    // Disable mint authority
    do_process_instruction(
        set_authority(
            &program_id,
            &mint_key,
            None,
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
    assert!(!mint.is_eligible_for_shared_liquidity()); // Not eligible due to zero supply

    // Mint some tokens
    let account_key = Pubkey::new_unique();
    let mut account = SolanaAccount::new(account_minimum_balance(), Account::LEN, &program_id);

    do_process_instruction(
        initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
        vec![&mut account, &mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    do_process_instruction(
        mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
        vec![&mut mint_account, &mut account],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(mint.is_eligible_for_shared_liquidity()); // Now eligible

    // Create another mint with too many decimals
    let mint_key2 = Pubkey::new_unique();
    let mut mint_account2 = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);

    do_process_instruction(
        initialize_mint(&program_id, &mint_key2, &owner_key, None, 10).unwrap(),
        vec![&mut mint_account2, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    let mint2 = Mint::unpack(&mint_account2.data).unwrap();
    assert!(!mint2.is_eligible_for_shared_liquidity()); // Not eligible due to too many decimals
}

#[test]
fn test_shared_liquidity_edge_cases() {
    let program_id = spl_token::id();
    let owner_key = Pubkey::new_unique();

    // Test mint with 0 decimals
    let mint_key = Pubkey::new_unique();
    let mut mint_account = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);
    let mut rent_sysvar = rent_sysvar();

    do_process_instruction(
        initialize_mint(&program_id, &mint_key, &owner_key, None, 0).unwrap(),
        vec![&mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    // Disable mint authority
    do_process_instruction(
        set_authority(
            &program_id,
            &mint_key,
            None,
            AuthorityType::MintTokens,
            &owner_key,
            &[],
        )
        .unwrap(),
        vec![&mut mint_account],
        &[Check::success()],
    )
    .unwrap();

    // Mint some tokens
    let account_key = Pubkey::new_unique();
    let mut account = SolanaAccount::new(account_minimum_balance(), Account::LEN, &program_id);

    do_process_instruction(
        initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
        vec![&mut account, &mut mint_account, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    do_process_instruction(
        mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
        vec![&mut mint_account, &mut account],
        &[Check::success()],
    )
    .unwrap();

    let mint = Mint::unpack(&mint_account.data).unwrap();
    assert!(mint.is_eligible_for_shared_liquidity()); // Eligible with 0 decimals

    // Test mint with 9 decimals (maximum allowed)
    let mint_key2 = Pubkey::new_unique();
    let mut mint_account2 = SolanaAccount::new(mint_minimum_balance(), Mint::LEN, &program_id);

    do_process_instruction(
        initialize_mint(&program_id, &mint_key2, &owner_key, None, 9).unwrap(),
        vec![&mut mint_account2, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    // Disable mint authority
    do_process_instruction(
        set_authority(
            &program_id,
            &mint_key2,
            None,
            AuthorityType::MintTokens,
            &owner_key,
            &[],
        )
        .unwrap(),
        vec![&mut mint_account2],
        &[Check::success()],
    )
    .unwrap();

    // Mint some tokens
    let account_key2 = Pubkey::new_unique();
    let mut account2 = SolanaAccount::new(account_minimum_balance(), Account::LEN, &program_id);

    do_process_instruction(
        initialize_account(&program_id, &account_key2, &mint_key2, &owner_key).unwrap(),
        vec![&mut account2, &mut mint_account2, &mut rent_sysvar],
        &[Check::success()],
    )
    .unwrap();

    do_process_instruction(
        mint_to(
            &program_id,
            &mint_key2,
            &account_key2,
            &owner_key,
            &[],
            1000,
        )
        .unwrap(),
        vec![&mut mint_account2, &mut account2],
        &[Check::success()],
    )
    .unwrap();

    let mint2 = Mint::unpack(&mint_account2.data).unwrap();
    assert!(mint2.is_eligible_for_shared_liquidity()); // Eligible with 9 decimals
}
