use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use super::{
    create_plan::Plan,
    create_subscription::{Subscription, SubscriptionState},
};

pub fn handle_uncancel_subscription(ctx: Context<UncancelSubscriptionParams>) -> Result<()> {
    // cancel will cancel the subscription but let it finish out the current term;
    let subscription_account = &mut ctx.accounts.subscription_account;
    subscription_account.state = SubscriptionState::Active;
    Ok(())
}

#[derive(Accounts)]
pub struct UncancelSubscriptionParams<'info> {
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        constraint = subscription_account.owner == payer.key(),
        constraint = subscription_account.state == SubscriptionState::PendingCancellation,
        bump,
    )]
    pub subscription_account: Account<'info, Subscription>,
    #[account(
        mut,
        seeds = [b"plan".as_ref(), plan_account.owner.key().as_ref(), plan_account.code.as_ref()],
        bump,
    )]
    pub plan_account: Account<'info, Plan>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
