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

/// Maximum number of hops for multi-hop swaps
pub const MAX_HOPS: usize = 5;

/// Minimum output amount threshold for swap paths
pub const MIN_OUTPUT_AMOUNT_THRESHOLD: u64 = 100;
