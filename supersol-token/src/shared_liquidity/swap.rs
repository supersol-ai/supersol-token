use super::constants::*;
use super::types::*;
use crate::state::{Account, Mint};
use solana_program_error::{ProgramError, ProgramResult};
use std::collections::{HashMap, HashSet};

pub struct SwapManager;

impl SwapManager {
    /// Execute a swap in a single pool
    pub fn execute_swap(
        pool: &mut SharedLiquidityPool,
        input_amount: u64,
        min_output_amount: u64,
        is_token_a_to_b: bool,
    ) -> ProgramResult<u64> {
        // Check if pool is paused
        if EmergencyManager::is_paused(pool)? {
            return Err(ProgramError::Custom(1)); // Pool is paused
        }

        // Calculate output amount
        let output_amount = if is_token_a_to_b {
            Self::calculate_swap_output(pool.reserve_a, pool.reserve_b, input_amount, pool.fee)?
        } else {
            Self::calculate_swap_output(pool.reserve_b, pool.reserve_a, input_amount, pool.fee)?
        };

        // Check minimum output
        if output_amount < min_output_amount {
            return Err(ProgramError::Custom(9)); // Slippage too high
        }

        // Update reserves
        if is_token_a_to_b {
            pool.reserve_a = pool
                .reserve_a
                .checked_add(input_amount)
                .ok_or(ProgramError::Overflow)?;
            pool.reserve_b = pool
                .reserve_b
                .checked_sub(output_amount)
                .ok_or(ProgramError::Overflow)?;
        } else {
            pool.reserve_b = pool
                .reserve_b
                .checked_add(input_amount)
                .ok_or(ProgramError::Overflow)?;
            pool.reserve_a = pool
                .reserve_a
                .checked_sub(output_amount)
                .ok_or(ProgramError::Overflow)?;
        }

        // Update fees for concentrated positions
        for (position_id, _) in &pool.enhanced_positions {
            ConcentratedLiquidityManager::update_fees(
                pool,
                *position_id,
                input_amount,
                output_amount,
            )?;
        }

        Ok(output_amount)
    }

    /// Find the best swap path for a multi-hop swap
    pub fn find_best_swap_path(
        pools: &[SharedLiquidityPool],
        input_token: Pubkey,
        output_token: Pubkey,
        input_amount: u64,
        min_output_amount: u64,
    ) -> ProgramResult<Option<SwapPath>> {
        let mut best_path = None;
        let mut best_output = 0;

        // Find all possible paths
        let paths = Self::find_all_paths(pools, input_token, output_token, MAX_HOPS)?;

        // Simulate each path
        for path in paths {
            let output = Self::simulate_swap_path(pools, &path, input_amount)?;

            if output > best_output {
                best_output = output;
                best_path = Some(path);
            }
        }

        // Check minimum output
        if best_output < min_output_amount {
            return Err(ProgramError::Custom(9)); // Slippage too high
        }

        Ok(best_path)
    }

    /// Execute a multi-hop swap
    pub fn execute_multi_hop_swap(
        pools: &mut [SharedLiquidityPool],
        path: &SwapPath,
        input_amount: u64,
        min_output_amount: u64,
    ) -> ProgramResult<u64> {
        let mut current_amount = input_amount;

        // Execute swaps through each pool in the path
        for (i, pool) in path.pools.iter().enumerate() {
            let is_token_a_to_b = i == 0 || path.pools[i - 1].token_b_mint == pool.token_a_mint;
            current_amount = Self::execute_swap(
                &mut pools[i],
                current_amount,
                if i == path.pools.len() - 1 {
                    min_output_amount
                } else {
                    0
                },
                is_token_a_to_b,
            )?;
        }

        Ok(current_amount)
    }

    /// Calculate swap output amount
    fn calculate_swap_output(
        reserve_in: u64,
        reserve_out: u64,
        amount_in: u64,
        fee: u64,
    ) -> ProgramResult<u64> {
        // Calculate amount in with fee
        let amount_in_with_fee = (amount_in as u128)
            .checked_mul(10000 - fee as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(10000)
            .ok_or(ProgramError::Overflow)?;

        // Calculate output amount using constant product formula
        let numerator = (amount_in_with_fee as u128)
            .checked_mul(reserve_out as u128)
            .ok_or(ProgramError::Overflow)?;
        let denominator = (reserve_in as u128)
            .checked_add(amount_in_with_fee)
            .ok_or(ProgramError::Overflow)?;

        numerator
            .checked_div(denominator)
            .ok_or(ProgramError::Overflow)
            .map(|amount| amount as u64)
    }

    /// Find all possible swap paths
    fn find_all_paths(
        pools: &[SharedLiquidityPool],
        input_token: Pubkey,
        output_token: Pubkey,
        max_hops: usize,
    ) -> ProgramResult<Vec<SwapPath>> {
        let mut paths = Vec::new();
        let mut visited = HashSet::new();

        Self::dfs_find_paths(
            pools,
            input_token,
            output_token,
            max_hops,
            &mut Vec::new(),
            &mut visited,
            &mut paths,
        )?;

        Ok(paths)
    }

    /// DFS helper for finding paths
    fn dfs_find_paths(
        pools: &[SharedLiquidityPool],
        current_token: Pubkey,
        target_token: Pubkey,
        remaining_hops: usize,
        current_path: &mut Vec<SharedLiquidityPool>,
        visited: &mut HashSet<Pubkey>,
        paths: &mut Vec<SwapPath>,
    ) -> ProgramResult {
        // Check if we've reached the target
        if current_token == target_token {
            paths.push(SwapPath {
                pools: current_path.clone(),
                expected_output: 0,
                price_impact: 0,
            });
            return Ok(());
        }

        // Check if we've used all hops
        if remaining_hops == 0 {
            return Ok(());
        }

        // Try each pool
        for pool in pools {
            // Skip if pool is paused
            if EmergencyManager::is_paused(pool)? {
                continue;
            }

            // Check if pool contains current token
            if pool.token_a_mint == current_token {
                if !visited.contains(&pool.token_b_mint) {
                    visited.insert(pool.token_b_mint);
                    current_path.push(pool.clone());
                    Self::dfs_find_paths(
                        pools,
                        pool.token_b_mint,
                        target_token,
                        remaining_hops - 1,
                        current_path,
                        visited,
                        paths,
                    )?;
                    current_path.pop();
                    visited.remove(&pool.token_b_mint);
                }
            } else if pool.token_b_mint == current_token {
                if !visited.contains(&pool.token_a_mint) {
                    visited.insert(pool.token_a_mint);
                    current_path.push(pool.clone());
                    Self::dfs_find_paths(
                        pools,
                        pool.token_a_mint,
                        target_token,
                        remaining_hops - 1,
                        current_path,
                        visited,
                        paths,
                    )?;
                    current_path.pop();
                    visited.remove(&pool.token_a_mint);
                }
            }
        }

        Ok(())
    }

    /// Simulate a swap path
    fn simulate_swap_path(
        pools: &[SharedLiquidityPool],
        path: &SwapPath,
        input_amount: u64,
    ) -> ProgramResult<u64> {
        let mut current_amount = input_amount;

        // Simulate swaps through each pool
        for (i, pool) in path.pools.iter().enumerate() {
            let is_token_a_to_b = i == 0 || path.pools[i - 1].token_b_mint == pool.token_a_mint;
            current_amount = Self::calculate_swap_output(
                if is_token_a_to_b {
                    pool.reserve_a
                } else {
                    pool.reserve_b
                },
                if is_token_a_to_b {
                    pool.reserve_b
                } else {
                    pool.reserve_a
                },
                current_amount,
                pool.fee,
            )?;
        }

        Ok(current_amount)
    }

    /// Calculate price impact for a swap
    pub fn calculate_price_impact(
        pool: &SharedLiquidityPool,
        input_amount: u64,
        output_amount: u64,
        is_token_a_to_b: bool,
    ) -> ProgramResult<u64> {
        // Calculate spot price
        let spot_price = Self::calculate_spot_price(pool, is_token_a_to_b)?;

        // Calculate execution price
        let execution_price = (output_amount as u128)
            .checked_mul(PRICE_PRECISION as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(input_amount as u128)
            .ok_or(ProgramError::Overflow)? as u64;

        // Calculate price impact
        let price_impact = if is_token_a_to_b {
            spot_price
                .checked_sub(execution_price)
                .ok_or(ProgramError::Overflow)?
        } else {
            execution_price
                .checked_sub(spot_price)
                .ok_or(ProgramError::Overflow)?
        };

        Ok(price_impact)
    }

    /// Calculate spot price for a pool
    fn calculate_spot_price(
        pool: &SharedLiquidityPool,
        is_token_a_to_b: bool,
    ) -> ProgramResult<u64> {
        if pool.reserve_b == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        let price = (pool.reserve_a as u128)
            .checked_mul(PRICE_PRECISION as u128)
            .ok_or(ProgramError::Overflow)?
            .checked_div(pool.reserve_b as u128)
            .ok_or(ProgramError::Overflow)? as u64;

        Ok(if is_token_a_to_b {
            price
        } else {
            PRICE_PRECISION
                .checked_div(price)
                .ok_or(ProgramError::Overflow)?
        })
    }
}
