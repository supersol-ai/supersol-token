use crate::state::{Account, Mint};
use solana_pubkey::Pubkey;

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

/// Swap path for multi-hop swaps
#[derive(Debug, Clone)]
pub struct SwapPath {
    pub pools: Vec<Pubkey>,
    pub expected_output: u64,
    pub price_impact: u64,
}
