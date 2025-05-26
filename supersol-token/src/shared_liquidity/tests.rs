#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;
    use std::str::FromStr;

    fn create_test_pool() -> SharedLiquidityPool {
        let token_a = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        let token_b = Pubkey::from_str("22222222222222222222222222222222").unwrap();
        let admin = Pubkey::from_str("33333333333333333333333333333333").unwrap();

        SharedLiquidityChecker::create_liquidity_pool(token_a, token_b, 30, admin).unwrap()
    }

    #[test]
    fn test_basic_pool_operations() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Test adding liquidity
        SharedLiquidityChecker::add_liquidity(&mut pool, 1000, 1000, owner).unwrap();
        assert_eq!(pool.reserve_a, 1000);
        assert_eq!(pool.reserve_b, 1000);

        // Test removing liquidity
        let (amount_a, amount_b) =
            SharedLiquidityChecker::remove_liquidity(&mut pool, 1, 500).unwrap();
        assert_eq!(amount_a, 500);
        assert_eq!(amount_b, 500);
        assert_eq!(pool.reserve_a, 500);
        assert_eq!(pool.reserve_b, 500);
    }

    #[test]
    fn test_concentrated_liquidity() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Test creating concentrated position
        let position_id = SharedLiquidityChecker::create_concentrated_position(
            &mut pool, owner, 1000, 1000, 90, 110,
        )
        .unwrap();
        assert_eq!(position_id, 1);

        // Test adding range
        SharedLiquidityChecker::add_range_to_position(&mut pool, position_id, 500, 500, 95, 105)
            .unwrap();

        // Test removing range
        let (amount_a, amount_b) =
            SharedLiquidityChecker::remove_range_from_position(&mut pool, position_id, 0).unwrap();
        assert_eq!(amount_a, 1000);
        assert_eq!(amount_b, 1000);
    }

    #[test]
    fn test_liquidity_mining() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Initialize mining
        SharedLiquidityChecker::initialize_mining(
            &mut pool,
            10000,
            Clock::get().unwrap().unix_timestamp,
            Clock::get().unwrap().unix_timestamp + 1000,
            100,
        )
        .unwrap();

        // Add liquidity
        SharedLiquidityChecker::add_liquidity(&mut pool, 1000, 1000, owner).unwrap();

        // Claim rewards
        let rewards = SharedLiquidityChecker::claim_mining_rewards(&mut pool, 1).unwrap();
        assert!(rewards > 0);
    }

    #[test]
    fn test_price_oracle() {
        let mut pool = create_test_pool();

        // Initialize oracle
        SharedLiquidityChecker::initialize_oracle(&mut pool, 60, 100).unwrap();

        // Update price
        SharedLiquidityChecker::update_oracle_price(&mut pool, 1000).unwrap();
        let price = PriceOracleManager::get_price(&pool).unwrap();
        assert_eq!(price, 1000);
    }

    #[test]
    fn test_emergency_pause() {
        let mut pool = create_test_pool();
        let admin = Pubkey::from_str("33333333333333333333333333333333").unwrap();

        // Initialize emergency pause
        SharedLiquidityChecker::initialize_emergency_pause(&mut pool, 3600, admin).unwrap();

        // Pause pool
        SharedLiquidityChecker::pause_pool(&mut pool, admin).unwrap();
        assert!(EmergencyManager::is_paused(&pool).unwrap());

        // Resume pool
        SharedLiquidityChecker::resume_pool(&mut pool, admin).unwrap();
        assert!(!EmergencyManager::is_paused(&pool).unwrap());
    }

    #[test]
    fn test_swaps() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Add initial liquidity
        SharedLiquidityChecker::add_liquidity(&mut pool, 1000, 1000, owner).unwrap();

        // Test single swap
        let output = SharedLiquidityChecker::execute_swap(&mut pool, 100, 90, true).unwrap();
        assert!(output >= 90);

        // Test multi-hop swap
        let mut pool2 = create_test_pool();
        let token_c = Pubkey::from_str("55555555555555555555555555555555").unwrap();
        pool2.token_b_mint = token_c;
        SharedLiquidityChecker::add_liquidity(&mut pool2, 1000, 1000, owner).unwrap();

        let pools = vec![pool, pool2];
        let path = SharedLiquidityChecker::find_best_swap_path(
            &pools,
            pool.token_a_mint,
            token_c,
            100,
            80,
        )
        .unwrap()
        .unwrap();

        let output =
            SharedLiquidityChecker::execute_multi_hop_swap(&mut pools, &path, 100, 80).unwrap();
        assert!(output >= 80);
    }

    #[test]
    fn test_edge_cases() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Test invalid price range
        assert!(SharedLiquidityChecker::create_concentrated_position(
            &mut pool, owner, 1000, 1000, 110, 90,
        )
        .is_err());

        // Test too many ranges
        let position_id = SharedLiquidityChecker::create_concentrated_position(
            &mut pool, owner, 1000, 1000, 90, 110,
        )
        .unwrap();

        for _ in 0..MAX_RANGES_PER_POSITION {
            SharedLiquidityChecker::add_range_to_position(
                &mut pool,
                position_id,
                100,
                100,
                95,
                105,
            )
            .unwrap();
        }

        assert!(SharedLiquidityChecker::add_range_to_position(
            &mut pool,
            position_id,
            100,
            100,
            95,
            105,
        )
        .is_err());

        // Test invalid swap amounts
        assert!(SharedLiquidityChecker::execute_swap(&mut pool, 0, 0, true,).is_err());

        // Test non-existent position
        assert!(SharedLiquidityChecker::remove_liquidity(&mut pool, 999, 100,).is_err());
    }

    #[test]
    fn test_concentrated_liquidity_advanced() {
        let mut pool = create_test_pool();
        let owner = Pubkey::from_str("44444444444444444444444444444444").unwrap();

        // Create initial position
        let position_id = SharedLiquidityChecker::create_concentrated_position(
            &mut pool, owner, 1000, 1000, 90, 110,
        )
        .unwrap();

        // Test range overlap protection
        assert!(SharedLiquidityChecker::add_range_to_position(
            &mut pool,
            position_id,
            500,
            500,
            95,
            105,
        )
        .is_err());

        // Test range rebalancing
        let target_ranges = vec![(85, 95), (95, 105), (105, 115)];
        SharedLiquidityChecker::rebalance_ranges(&mut pool, position_id, target_ranges).unwrap();
        assert_eq!(pool.enhanced_positions[&position_id].ranges.len(), 3);

        // Test range fee optimization
        SharedLiquidityChecker::optimize_range_fees(&mut pool, position_id).unwrap();
        assert!(pool.enhanced_positions[&position_id].ranges.len() <= 3);

        // Test optimal range calculation
        let current_price = 100;
        let optimal_ranges =
            SharedLiquidityChecker::get_optimal_ranges(&pool, current_price, 3).unwrap();
        assert_eq!(optimal_ranges.len(), 3);
        assert!(optimal_ranges[0].0 < optimal_ranges[0].1);
        assert!(optimal_ranges[1].0 < optimal_ranges[1].1);
        assert!(optimal_ranges[2].0 < optimal_ranges[2].1);
    }
}
