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
    state::{AccountState, Mint},
};

mod common;
mod flash_loan_tests;

pub use common::*;

pub struct TestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub pool_authority: Keypair,
    pub token_a_mint: Keypair,
    pub token_b_mint: Keypair,
}

impl TestContext {
    pub async fn new() -> Self {
        let program_id = Pubkey::new_unique();
        let payer = Keypair::new();
        let pool_authority = Keypair::new();
        let token_a_mint = Keypair::new();
        let token_b_mint = Keypair::new();

        let mut program_test = ProgramTest::new(
            "supersol_token",
            program_id,
            processor!(supersol_token::processor::Processor::process),
        );

        // Add accounts
        program_test.add_account(
            payer.pubkey(),
            Account {
                lamports: 1_000_000_000,
                owner: solana_program::system_program::id(),
                ..Account::default()
            },
        );

        // Initialize token mints
        let token_a_mint_account = Account {
            lamports: 1_000_000_000,
            owner: program_id,
            data: bincode::serialize(&Mint {
                mint_authority: Some(pool_authority.pubkey()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            })
            .unwrap(),
            ..Account::default()
        };

        let token_b_mint_account = Account {
            lamports: 1_000_000_000,
            owner: program_id,
            data: bincode::serialize(&Mint {
                mint_authority: Some(pool_authority.pubkey()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            })
            .unwrap(),
            ..Account::default()
        };

        program_test.add_account(token_a_mint.pubkey(), token_a_mint_account);
        program_test.add_account(token_b_mint.pubkey(), token_b_mint_account);

        Self {
            program_id,
            payer,
            pool_authority,
            token_a_mint,
            token_b_mint,
        }
    }

    pub async fn create_liquidity_pool(&self, client: &mut ProgramTestContext) -> Pubkey {
        let pool_account = Keypair::new();
        let pool_size = 1000;

        let create_pool_ix = instruction::create_liquidity_pool(
            self.program_id,
            self.payer.pubkey(),
            self.pool_authority.pubkey(),
            self.token_a_mint.pubkey(),
            self.token_b_mint.pubkey(),
            pool_size,
        )
        .unwrap();

        let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[create_pool_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &self.pool_authority],
            recent_blockhash,
        );

        client
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        pool_account.pubkey()
    }
}

pub async fn setup_test_account(
    client: &mut ProgramTestContext,
    owner: &Pubkey,
    lamports: u64,
) -> Keypair {
    let account = Keypair::new();
    let rent = client
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(0);

    let create_account_ix = system_instruction::create_account(
        owner,
        &account.pubkey(),
        rent + lamports,
        0,
        &solana_program::system_program::id(),
    );

    let recent_blockhash = client.banks_client.get_latest_blockhash().await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix],
        Some(owner),
        &[&account],
        recent_blockhash,
    );

    client
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    account
}
