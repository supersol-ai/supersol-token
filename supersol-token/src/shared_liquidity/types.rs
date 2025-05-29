use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

/// Maximum number of decimals allowed for shared liquidity tokens
pub const MAX_SHARED_LIQUIDITY_DECIMALS: u8 = 9;

/// Minimum supply required for shared liquidity tokens
pub const MIN_SHARED_LIQUIDITY_SUPPLY: u64 = 1_000_000; // 1M tokens minimum

/// Maximum supply allowed for shared liquidity tokens
pub const MAX_SHARED_LIQUIDITY_SUPPLY: u64 = 1_000_000_000_000; // 1T tokens maximum

/// Maximum number of ranges per position
pub const MAX_RANGES_PER_POSITION: u8 = 10;

/// Minimum range width in basis points (0.01%)
pub const MIN_RANGE_WIDTH: u32 = 1;

/// Maximum range width in basis points (100%)
pub const MAX_RANGE_WIDTH: u32 = 10000;

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

/// Price oracle configuration for liquidity pool
#[derive(Debug, Clone, PartialEq)]
pub struct PriceOracle {
    /// Last observed price of token A in terms of token B
    pub last_price: f64,
    /// Timestamp of last price update
    pub last_update: i64,
    /// Minimum time between price updates (in seconds)
    pub min_update_interval: i64,
    /// Maximum allowed price deviation (in basis points)
    pub max_price_deviation: u16,
    /// Whether the oracle is active
    pub is_active: bool,
}

/// Liquidity mining configuration
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidityMining {
    /// Total rewards allocated for mining
    pub total_rewards: u64,
    /// Remaining rewards to be distributed
    pub remaining_rewards: u64,
    /// Start timestamp of mining program
    pub start_time: i64,
    /// End timestamp of mining program
    pub end_time: i64,
    /// Reward rate per second
    pub reward_rate: u64,
    /// Total liquidity provider shares
    pub total_shares: u64,
    /// Whether mining is active
    pub is_active: bool,
}

/// Liquidity provider position
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidityPosition {
    /// Provider's share of the pool
    pub shares: u64,
    /// Last reward claim timestamp
    pub last_claim_time: i64,
    /// Accumulated rewards
    pub accumulated_rewards: u64,
    /// Position owner
    pub owner: Pubkey,
    /// Position ID
    pub id: u64,
}

/// Emergency pause configuration
#[derive(Debug, Clone, PartialEq)]
pub struct EmergencyPause {
    /// Whether the pool is paused
    pub is_paused: bool,
    /// Timestamp when the pool was paused
    pub pause_timestamp: i64,
    /// Maximum pause duration in seconds
    pub max_pause_duration: i64,
    /// Administrator public key
    pub admin: Pubkey,
}

/// Concentrated liquidity range
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidityRange {
    /// Lower price bound in basis points
    pub lower_price: u32,
    /// Upper price bound in basis points
    pub upper_price: u32,
    /// Liquidity amount in this range
    pub liquidity: u64,
    /// Fees collected in this range
    pub fees_collected: u64,
    /// Last update timestamp
    pub last_update: i64,
}

/// Enhanced liquidity position with ranges
#[derive(Debug, Clone, PartialEq)]
pub struct EnhancedLiquidityPosition {
    /// Base position data
    pub base_position: LiquidityPosition,
    /// Active liquidity ranges
    pub ranges: Vec<LiquidityRange>,
    /// Position ID
    pub position_id: u64,
    /// Creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
}

/// Liquidity pool configuration for shared liquidity tokens
#[derive(Debug, Clone, PartialEq)]
pub struct SharedLiquidityPool {
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
    /// Flash loan fee in basis points (e.g., 9 = 0.09%)
    pub flash_loan_fee: u16,
    /// Maximum flash loan amount as percentage of pool balance (e.g., 50 = 50%)
    pub max_flash_loan_percentage: u8,
    /// Price oracle configuration
    pub price_oracle: PriceOracle,
    /// Emergency pause configuration
    pub emergency_pause: EmergencyPause,
    /// Liquidity provider positions
    pub positions: std::collections::HashMap<Pubkey, LiquidityPosition>,
    /// Enhanced liquidity positions
    pub enhanced_positions: std::collections::HashMap<u64, EnhancedLiquidityPosition>,
    /// Next position ID
    pub next_position_id: u64,
    /// Current price in basis points
    pub current_price: u32,
    /// Price history for range calculations
    pub price_history: Vec<(i64, u32)>,
}

/// Swap path for multi-hop swaps
#[derive(Debug, Clone, PartialEq)]
pub struct SwapPath {
    /// Pool addresses in the path
    pub pools: Vec<Pubkey>,
    /// Expected output amount
    pub expected_output: u64,
    /// Price impact in basis points
    pub price_impact: u64,
}
