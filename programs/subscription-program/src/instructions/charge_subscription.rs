use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};
use solana_program::pubkey;

use crate::SubscriptionErrors;

use super::{
    create_plan::Plan,
    create_subscription::{Subscription, SubscriptionState},
};

pub fn handle_charge_subscription(ctx: Context<ChargeSubscriptionParams>) -> Result<()> {
    let plan_account = &mut ctx.accounts.plan_account;
    let subscription_account = &mut ctx.accounts.subscription_account;
    let plan_token_account = &mut ctx.accounts.plan_token_account;
    let subscriber_token_account = &mut ctx.accounts.subscriber_token_account;
    let deployer_token_account = &mut ctx.accounts.deployer_token_account;
    let owner_token_account = &ctx.accounts.owner_token_account;

    let current = Clock::get()?.unix_timestamp;
    if current < subscription_account.next_term_date {
        return Err(SubscriptionErrors::SubscriptionNotReady.into());
    }
    if subscriber_token_account.amount < plan_account.price {
        subscription_account.state = SubscriptionState::PastDue;
        return Ok(());
    }
    let transfer_accounts = Transfer {
        from: subscriber_token_account.to_account_info().clone(),
        to: plan_token_account.to_account_info().clone(),
        authority: subscription_account.to_account_info().clone(),
    };

    let (_pda, subscription_bump) = Pubkey::find_program_address(
        &[
            b"subscription".as_ref(),
            subscription_account.owner.key().as_ref(),
            plan_account.key().as_ref(),
        ],
        ctx.program_id,
    );
    let token_program = &ctx.accounts.token_program;
    let cpi_program: AccountInfo<'_> = token_program.to_account_info();
    transfer(
        CpiContext::new_with_signer(
            cpi_program.clone(),
            transfer_accounts,
            &[&[
                b"subscription".as_ref(),
                subscription_account.owner.key().as_ref(),
                plan_account.key().as_ref(),
                &[subscription_bump],
            ]],
        ),
        plan_account.price,
    )?;
    let payout_accounts = Transfer {
        from: plan_token_account.to_account_info().clone(),
        to: owner_token_account.to_account_info().clone(),
        authority: plan_account.to_account_info().clone(),
    };
    let tax = ((plan_account.price as f32) * 0.03) as u64;
    let (_pda, plan_bump) = Pubkey::find_program_address(
        &[
            b"plan".as_ref(),
            plan_account.owner.key().as_ref(),
            plan_account.code.as_ref(),
        ],
        ctx.program_id,
    );
    // payout the owner of the plan for the previous charge on the subscription;
    transfer(
        CpiContext::new_with_signer(
            cpi_program.clone(),
            payout_accounts,
            &[&[
                b"plan".as_ref(),
                plan_account.owner.key().as_ref(),
                plan_account.code.as_ref(),
                &[plan_bump],
            ]],
        ),
        plan_account.price - tax,
    )?;
    let tax_accounts = Transfer {
        from: plan_token_account.to_account_info().clone(),
        to: deployer_token_account.to_account_info().clone(),
        authority: plan_account.to_account_info().clone(),
    };
    transfer(
        CpiContext::new_with_signer(
            cpi_program.clone(),
            tax_accounts,
            &[&[
                b"plan".as_ref(),
                plan_account.owner.key().as_ref(),
                plan_account.code.as_ref(),
                &[plan_bump],
            ]],
        ),
        tax,
    )?;
    subscription_account.next_term_date += plan_account.term_in_seconds as i64;
    Ok(())
}

#[derive(Accounts)]
pub struct ChargeSubscriptionParams<'info> {
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        bump,
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
        constraint = subscriber_token_account.mint == plan_account.token_mint,
        constraint = subscriber_token_account.owner == subscription_account.owner.key(),
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = owner_token_account.mint == plan_account.token_mint,
        constraint = owner_token_account.owner == plan_account.owner.key(),
    )]
    pub owner_token_account: Account<'info, TokenAccount>,
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
