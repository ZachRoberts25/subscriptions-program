use anchor_lang::prelude::*;
use anchor_spl::token::{revoke, transfer, Revoke, Token, TokenAccount, Transfer};
use solana_program::pubkey;

use super::{create_plan::Plan, create_subscription::Subscription};

pub fn handle_close_subscription(ctx: Context<CloseSubscriptionParams>) -> Result<()> {
    // close immediately, closes the subscription account and refunds the user for the remaining time;
    let subscription_account = &mut ctx.accounts.subscription_account;
    let plan_account = &mut ctx.accounts.plan_account;
    let plan_token_account = &mut ctx.accounts.plan_token_account;
    let payer = &ctx.accounts.payer;
    let payer_token_account = &mut ctx.accounts.payer_token_account;
    let plan_owner_token_account = &ctx.accounts.plan_owner_token_account;
    let deployer_token_account = &ctx.accounts.deployer_token_account;
    let token_program = &ctx.accounts.token_program;
    let current = Clock::get()?.unix_timestamp;
    plan_account.active_subscriptions.checked_sub(1).or(Some(0));
    // the subscription end date is in the future so the user needs a refund for the remaining time;
    if current < subscription_account.next_term_date {
        let term_seconds = plan_account.term_in_seconds;
        msg!("term seconds {}", term_seconds);
        let time_diff = subscription_account.next_term_date - current;
        msg!("time diff {}", time_diff);
        let percentage = time_diff as f64 / term_seconds as f64;
        msg!("percentage {}", percentage);
        let refund = (plan_account.price as f64 * percentage) as u64;
        msg!("refund {}", refund);
        let payer_payout = Transfer {
            from: plan_token_account.to_account_info().clone(),
            to: payer_token_account.to_account_info().clone(),
            authority: plan_account.to_account_info().clone(),
        };
        let plan_account_owner_key = plan_account.owner.key();
        let seeds = &[
            b"plan".as_ref(),
            plan_account_owner_key.as_ref(),
            plan_account.code.as_ref(),
        ];
        let (_pda, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info().clone(),
                payer_payout,
                &[&[
                    b"plan".as_ref(),
                    plan_account_owner_key.as_ref(),
                    plan_account.code.as_ref(),
                    &[bump],
                ]],
            ),
            refund,
        )?;

        let owner_payout = Transfer {
            from: plan_token_account.to_account_info().clone(),
            to: plan_owner_token_account.to_account_info().clone(),
            authority: plan_account.to_account_info().clone(),
        };
        let total = plan_account.price - refund;
        let tax = ((total as f64) * 0.03) as u64;
        transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info().clone(),
                owner_payout,
                &[&[
                    b"plan".as_ref(),
                    plan_account_owner_key.as_ref(),
                    plan_account.code.as_ref(),
                    &[bump],
                ]],
            ),
            total - tax,
        )?;

        let tax_accounts = Transfer {
            from: plan_token_account.to_account_info().clone(),
            to: deployer_token_account.to_account_info().clone(),
            authority: plan_account.to_account_info().clone(),
        };
        transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info().clone(),
                tax_accounts,
                &[&[
                    b"plan".as_ref(),
                    plan_account_owner_key.as_ref(),
                    plan_account.code.as_ref(),
                    &[bump],
                ]],
            ),
            tax,
        )?;
    }

    let revoke_accounts = Revoke {
        authority: payer.to_account_info().clone(),
        source: payer_token_account.to_account_info().clone(),
    };
    revoke(CpiContext::new(
        token_program.to_account_info().clone(),
        revoke_accounts,
    ))?;
    Ok(())
}

#[derive(Accounts)]
pub struct CloseSubscriptionParams<'info> {
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        constraint = subscription_account.owner == payer.key(),
        bump,
        close = payer,
    )]
    pub subscription_account: Account<'info, Subscription>,
    #[account(
        mut,
        seeds = [b"plan".as_ref(), plan_account.owner.key().as_ref(), plan_account.code.as_ref()],
        bump,
    )]
    pub plan_account: Account<'info, Plan>,
    #[account(
        mut,
        constraint = plan_token_account.mint == plan_account.token_mint,
        constraint = plan_token_account.owner == plan_account.key(),
    )]
    pub plan_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = payer_token_account.mint == plan_account.token_mint,
        constraint = payer_token_account.owner == subscription_account.owner.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = plan_owner_token_account.mint == plan_account.token_mint,
        constraint = plan_owner_token_account.owner == plan_account.owner.key(),
    )]
    pub plan_owner_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = deployer_token_account.mint == plan_account.token_mint,
        constraint = deployer_token_account.owner == pubkey!("8mw8QFoqRffuYtwVDw4QD6eEfg1wEpYB24oL44toeZxy"),
    )]
    pub deployer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
