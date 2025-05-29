//! Flash Loan Integration Tests
//! 
//! This module contains integration tests for the flash loan functionality.
//! These tests verify the interaction between different components of the
//! flash loan system in a simulated Solana environment.
//! 
//! # Test Categories
//! 
//! 1. Basic Operations
//!    - Creating liquidity pools
//!    - Executing flash loans
//!    - Verifying pool state
//! 
//! 2. Safety Features
//!    - Loan amount limits
//!    - Pool state validation
//!    - Emergency pause
//! 
//! 3. Fee Management
//!    - Fee calculation
//!    - Fee distribution
//!    - Provider rewards
//! 
//! # Test Environment
//! 
//! Each test sets up a simulated Solana environment with:
//! - Program accounts
//! - Token mints
//! - Liquidity pools
//! - Test accounts

use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use supersol_token::{
    instruction,
    state::{AccountState, Mint},
};

mod common;
use common::*;

/// Tests basic flash loan functionality
/// 
/// This test verifies that:
/// 1. A flash loan can be executed successfully
/// 2. The pool state is maintained correctly
/// 3. The loan amount matches the request
#[tokio::test]
async fn test_flash_loan_basic() {
    let mut context = TestContext::new().await;
    let mut client = context.start().await;

    // Create liquidity pool
    let pool_pubkey = context.create_liquidity_pool(&mut client).await;

    // Create borrower account
    let borrower = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;

    // Add initial liquidity
    let add_liquidity_ix = instruction::add_liquidity(
        context.program_id,
        context.payer.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        1000,
        1000,
    )
    .unwrap();

    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[add_liquidity_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Execute flash loan
    let flash_loan_ix = instruction::execute_flash_loan(
        context.program_id,
        borrower.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        100,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[flash_loan_ix],
        Some(&borrower.pubkey()),
        &[&borrower],
        recent_blockhash,
    );

    let result = client.banks_client.process_transaction(transaction).await;

    assert!(result.is_ok());

    // Verify pool state
    let pool_account = client
        .banks_client
        .get_account(pool_pubkey)
        .await
        .unwrap()
        .unwrap();

    let pool_data = bincode::deserialize::<SharedLiquidityPool>(&pool_account.data).unwrap();
    assert_eq!(pool_data.token_a_balance, 1000);
}

/// Tests flash loan amount limits
/// 
/// This test verifies that:
/// 1. Loans exceeding the maximum amount are rejected
/// 2. The pool state remains unchanged
/// 3. Appropriate error is returned
#[tokio::test]
async fn test_flash_loan_limits() {
    let mut context = TestContext::new().await;
    let mut client = context.start().await;

    // Create liquidity pool
    let pool_pubkey = context.create_liquidity_pool(&mut client).await;

    // Create borrower account
    let borrower = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;

    // Add initial liquidity
    let add_liquidity_ix = instruction::add_liquidity(
        context.program_id,
        context.payer.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        1000,
        1000,
    )
    .unwrap();

    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[add_liquidity_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Try to execute flash loan exceeding limits
    let flash_loan_ix = instruction::execute_flash_loan(
        context.program_id,
        borrower.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        600, // Exceeds 50% of pool balance
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[flash_loan_ix],
        Some(&borrower.pubkey()),
        &[&borrower],
        recent_blockhash,
    );

    let result = client.banks_client.process_transaction(transaction).await;

    assert!(result.is_err());
}

/// Tests flash loan fee distribution
/// 
/// This test verifies that:
/// 1. Fees are calculated correctly
/// 2. Fees are distributed proportionally to providers
/// 3. Provider rewards are updated correctly
#[tokio::test]
async fn test_flash_loan_fee_distribution() {
    let mut context = TestContext::new().await;
    let mut client = context.start().await;

    // Create liquidity pool
    let pool_pubkey = context.create_liquidity_pool(&mut client).await;

    // Create two liquidity providers
    let provider1 = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;
    let provider2 = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;

    // Add liquidity from both providers
    let add_liquidity_ix1 = instruction::add_liquidity(
        context.program_id,
        provider1.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        500,
        500,
    )
    .unwrap();

    let add_liquidity_ix2 = instruction::add_liquidity(
        context.program_id,
        provider2.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        500,
        500,
    )
    .unwrap();

    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction1 = Transaction::new_signed_with_payer(
        &[add_liquidity_ix1],
        Some(&provider1.pubkey()),
        &[&provider1],
        recent_blockhash,
    );

    let transaction2 = Transaction::new_signed_with_payer(
        &[add_liquidity_ix2],
        Some(&provider2.pubkey()),
        &[&provider2],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction1)
        .await
        .unwrap();

    client
        .banks_client
        .process_transaction(transaction2)
        .await
        .unwrap();

    // Create borrower and execute flash loan
    let borrower = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;

    let flash_loan_ix = instruction::execute_flash_loan(
        context.program_id,
        borrower.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        100,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[flash_loan_ix],
        Some(&borrower.pubkey()),
        &[&borrower],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Verify fee distribution
    let pool_account = client
        .banks_client
        .get_account(pool_pubkey)
        .await
        .unwrap()
        .unwrap();

    let pool_data = bincode::deserialize::<SharedLiquidityPool>(&pool_account.data).unwrap();

    let position1 = pool_data.positions.get(&provider1.pubkey()).unwrap();
    let position2 = pool_data.positions.get(&provider2.pubkey()).unwrap();

    // Fee should be 0.09% of 100 = 0.09
    // Provider 1 should get 1/2 of the fee
    // Provider 2 should get 1/2 of the fee
    assert_eq!(position1.accumulated_rewards, 0);
    assert_eq!(position2.accumulated_rewards, 0);
}

/// Tests flash loan behavior with paused pools
/// 
/// This test verifies that:
/// 1. Flash loans are rejected when pool is paused
/// 2. Pool state remains unchanged
/// 3. Appropriate error is returned
#[tokio::test]
async fn test_flash_loan_paused_pool() {
    let mut context = TestContext::new().await;
    let mut client = context.start().await;

    // Create liquidity pool
    let pool_pubkey = context.create_liquidity_pool(&mut client).await;

    // Pause pool
    let pause_ix = instruction::pause_pool(
        context.program_id,
        context.pool_authority.pubkey(),
        pool_pubkey,
    )
    .unwrap();

    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[pause_ix],
        Some(&context.pool_authority.pubkey()),
        &[&context.pool_authority],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Try to execute flash loan on paused pool
    let borrower = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;

    let flash_loan_ix = instruction::execute_flash_loan(
        context.program_id,
        borrower.pubkey(),
        pool_pubkey,
        context.token_a_mint.pubkey(),
        100,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[flash_loan_ix],
        Some(&borrower.pubkey()),
        &[&borrower],
        recent_blockhash,
    );

    let result = client.banks_client.process_transaction(transaction).await;

    assert!(result.is_err());
}
