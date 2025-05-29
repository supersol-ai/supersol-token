//! Common Test Utilities
//!
//! This module provides common utilities for integration tests.
//! It includes functionality for setting up test environments,
//! creating test accounts, and managing test contexts.
//!
//! # Components
//!
//! 1. TestContext
//!    - Manages program state
//!    - Handles account creation
//!    - Provides test utilities
//!
//! 2. Helper Functions
//!    - Account setup
//!    - Transaction processing
//!    - State verification
//!
//! # Usage
//!
//! ```rust
//! let context = TestContext::new().await;
//! let client = context.start().await;
//!
//! // Create test accounts
//! let account = setup_test_account(&mut client, &context.payer.pubkey(), 1000).await;
//!
//! // Create liquidity pool
//! let pool = context.create_liquidity_pool(&mut client).await;
//! ```

use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use supersol_token::{
    instruction,
    state::{AccountState, Mint, SharedLiquidityPool},
};

/// Test context for managing program state and accounts
pub struct TestContext {
    /// Program ID for the test instance
    pub program_id: Pubkey,
    /// Payer account for transactions
    pub payer: Keypair,
    /// Pool authority for pool management
    pub pool_authority: Keypair,
    /// Token A mint account
    pub token_a_mint: Keypair,
    /// Token B mint account
    pub token_b_mint: Keypair,
}

impl TestContext {
    /// Creates a new test context
    ///
    /// Initializes all necessary accounts and program state
    /// for running integration tests.
    pub async fn new() -> Self {
        let program_id = Pubkey::new_unique();
        let payer = Keypair::new();
        let pool_authority = Keypair::new();
        let token_a_mint = Keypair::new();
        let token_b_mint = Keypair::new();

        Self {
            program_id,
            payer,
            pool_authority,
            token_a_mint,
            token_b_mint,
        }
    }

    /// Starts the test environment
    ///
    /// Sets up the program test context with all necessary
    /// accounts and configurations.
    pub async fn start(&self) -> ProgramTestContext {
        let mut program_test = ProgramTest::new(
            "supersol_token",
            self.program_id,
            processor!(supersol_token::processor::Processor::process),
        );

        // Add program accounts
        program_test.add_account(
            self.program_id,
            Account {
                lamports: 1000000000,
                owner: self.program_id,
                ..Account::default()
            },
        );

        // Add payer account
        program_test.add_account(
            self.payer.pubkey(),
            Account {
                lamports: 1000000000,
                owner: solana_program::system_program::id(),
                ..Account::default()
            },
        );

        // Add token mints
        program_test.add_account(
            self.token_a_mint.pubkey(),
            Account {
                lamports: 1000000000,
                owner: self.program_id,
                data: bincode::serialize(&Mint {
                    mint_authority: Some(self.pool_authority.pubkey()),
                    supply: 0,
                    decimals: 9,
                    is_initialized: true,
                    freeze_authority: None,
                })
                .unwrap(),
                ..Account::default()
            },
        );

        program_test.add_account(
            self.token_b_mint.pubkey(),
            Account {
                lamports: 1000000000,
                owner: self.program_id,
                data: bincode::serialize(&Mint {
                    mint_authority: Some(self.pool_authority.pubkey()),
                    supply: 0,
                    decimals: 9,
                    is_initialized: true,
                    freeze_authority: None,
                })
                .unwrap(),
                ..Account::default()
            },
        );

        program_test.start_with_context().await
    }

    /// Creates a new liquidity pool
    ///
    /// # Arguments
    ///
    /// * `client` - The program test context
    ///
    /// # Returns
    ///
    /// * `Pubkey` - The public key of the created pool
    pub async fn create_liquidity_pool(&self, client: &mut ProgramTestContext) -> Pubkey {
        let pool_keypair = Keypair::new();
        let pool_pubkey = pool_keypair.pubkey();

        let create_pool_ix = instruction::create_pool(
            self.program_id,
            self.pool_authority.pubkey(),
            pool_pubkey,
            self.token_a_mint.pubkey(),
            self.token_b_mint.pubkey(),
        )
        .unwrap();

        let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[create_pool_ix],
            Some(&self.pool_authority.pubkey()),
            &[&self.pool_authority, &pool_keypair],
            recent_blockhash,
        );

        client
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        pool_pubkey
    }
}

/// Sets up a test account with initial balance
///
/// # Arguments
///
/// * `client` - The program test context
/// * `payer` - The payer account
/// * `lamports` - Initial balance in lamports
///
/// # Returns
///
/// * `Keypair` - The created account
pub async fn setup_test_account(
    client: &mut ProgramTestContext,
    payer: &Pubkey,
    lamports: u64,
) -> Keypair {
    let account = Keypair::new();
    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            payer,
            &account.pubkey(),
            lamports,
        )],
        Some(payer),
        &[&client.payer_infos[0].keypair],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    account
}
