use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::sysvar::{clock::Clock, Sysvar};

declare_id!("YourProgramIdHere"); // Replace with your actual program ID after deployment

#[program]
pub mod staking_contract {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
        let staking_account = &mut ctx.accounts.staking_account;
        staking_account.total_staked = 0;
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> ProgramResult {
        let staking_account = &mut ctx.accounts.staking_account;
        let user = &mut ctx.accounts.user;
        let user_staking_info = &mut ctx.accounts.user_staking_info;

        // Transfer SOL from the user's account to the staking_account
        let ix = system_instruction::transfer(
            &user.to_account_info().key,
            &staking_account.to_account_info().key,
            amount,
        );
        invoke(
            &ix,
            &[
                user.to_account_info(),
                staking_account.to_account_info(),
            ],
        )?;

        // Update the total_staked and user's staked amount
        staking_account.total_staked += amount;
        user_staking_info.amount_staked += amount;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> ProgramResult {
        let staking_account = &mut ctx.accounts.staking_account;
        let user = &mut ctx.accounts.user;
        let user_staking_info = &mut ctx.accounts.user_staking_info;

        // Ensure the user has enough staked to unstake the desired amount
        if user_staking_info.amount_staked < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer SOL from the staking_account back to the user's account
        let ix = system_instruction::transfer(
            &staking_account.to_account_info().key,
            &user.to_account_info().key,
            amount,
        );
        invoke(
            &ix,
            &[
                staking_account.to_account_info(),
                user.to_account_info(),
            ],
        )?;

        // Update the total_staked and user's staked amount
        staking_account.total_staked -= amount;
        user_staking_info.amount_staked -= amount;

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> ProgramResult {
        let staking_account = &mut ctx.accounts.staking_account;
        let user_staking_info = &mut ctx.accounts.user_staking_info;
        let clock = Clock::get().unwrap();

        // Calculate the time difference in seconds since the last claim
        let time_staked = clock.unix_timestamp - user_staking_info.last_reward_claim;
        if time_staked <= 0 {
            return Err(ProgramError::InvalidArgument);
        }

        // Calculate the rewards
        let apy = staking_account.apy as f64 / 100.0;
        let days_staked = time_staked as f64 / 86_400.0; // seconds in a day
        let raw_rewards = user_staking_info.amount_staked as f64 * apy / 365.0 * days_staked;

        // Apply commission deduction
        let commission = staking_account.commission as f64 / 100.0;
        let rewards_after_commission = raw_rewards * (1.0 - commission);

        // Transfer the rewards from the staking_account to the user's account
        let ix = system_instruction::transfer(
            &staking_account.to_account_info().key,
            &user_staking_info.to_account_info().key,
            rewards_after_commission as u64,
        );
        invoke(
            &ix,
            &[
                staking_account.to_account_info(),
                user_staking_info.to_account_info(),
            ],
        )?;

        // Update the last_reward_claim to the current timestamp
        user_staking_info.last_reward_claim = clock.unix_timestamp;

        Ok(())
    }
}

#[account]
pub struct StakingAccount {
    pub total_staked: u64,
    pub apy: u8, // APY as a percentage, for example, 7 for 7% APY
    pub commission: u8, // Commission as a percentage, for example, 5 for 5% commission
    pub lock_up_period: i64, // Lock-up period in seconds
    pub minimum_stake: u64, // Minimum stake amount in lamports
    pub staking_start_time: i64, // Unix timestamp when staking started
    pub staking_cap: u64, // Maximum staking cap in lamports
    pub total_rewards: u64, // Total rewards distributed
    // Additional fields as needed
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8 + 40)] // Adjusted space for additional fields
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: This is only used to transfer SOL, so we don't need to deserialize it.
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub user_staking_info: Account<'info, UserStakingInfo>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub user_staking_info: Account<'info, UserStakingInfo>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user_staking_info: Account<'info, UserStakingInfo>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct UserStakingInfo {
    pub user: Pubkey,
    pub amount_staked: u64,
    pub last_reward_claim: i64, // Unix timestamp of the last claim
    pub staking_start_time: i64, // Unix timestamp when the user started staking
    pub rewards_accrued: u64, // Total rewards that have been accrued
    pub unstaking_start_time: i64, // Unix timestamp when the user initiated unstaking
    
}