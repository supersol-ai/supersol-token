use {
    crate::{
        instruction::{initialize_account, initialize_mint},
        state::{Account, Mint},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, instruction::Instruction,
        program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
    },
    spl_token::id,
};

pub struct SolanaAccount {
    pub key: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl SolanaAccount {
    pub fn new(lamports: u64, data_len: usize, owner: &Pubkey) -> Self {
        Self {
            key: Pubkey::new_unique(),
            lamports,
            data: vec![0; data_len],
            owner: *owner,
            executable: false,
            rent_epoch: 0,
        }
    }
}

pub struct Check {
    pub success: bool,
}

impl Check {
    pub fn success() -> Self {
        Self { success: true }
    }
}

pub fn do_process_instruction(
    instruction: Instruction,
    accounts: &mut [&mut SolanaAccount],
    checks: &[Check],
) -> ProgramResult {
    let mut account_infos: Vec<AccountInfo> = accounts
        .iter()
        .map(|account| {
            AccountInfo::new(
                &account.key,
                account.executable,
                false,
                &mut account.lamports,
                &mut account.data,
                &account.owner,
                false,
                account.rent_epoch,
            )
        })
        .collect();

    let result = crate::processor::Processor::process(&id(), &account_infos, &instruction.data);

    for check in checks {
        if check.success {
            assert!(result.is_ok());
        }
    }

    result
}

pub fn mint_minimum_balance() -> u64 {
    Rent::default().minimum_balance(Mint::LEN)
}

pub fn account_minimum_balance() -> u64 {
    Rent::default().minimum_balance(Account::LEN)
}

pub fn rent_sysvar() -> SolanaAccount {
    let rent = Rent::default();
    let mut data = vec![0; rent.try_serialize().unwrap().len()];
    rent.serialize(&mut data).unwrap();
    SolanaAccount {
        key: solana_program::sysvar::rent::id(),
        lamports: 0,
        data,
        owner: solana_program::sysvar::id(),
        executable: false,
        rent_epoch: 0,
    }
}
