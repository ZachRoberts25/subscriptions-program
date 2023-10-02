use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Approve, approve, Transfer, transfer};

use super::create_plan::Plan;


pub fn handle_create_subscription(
    ctx: Context<CreateSubscriptionParams>,
    data: CreateSubscriptionData,
) -> Result<()> {
    let plan_account = &mut ctx.accounts.plan_account;
    let subscription_account = &mut ctx.accounts.subscription_account;
    let payer_token_account = &mut ctx.accounts.payer_token_account;
    let plan_token_account = &mut ctx.accounts.plan_token_account;
    let payer = &mut ctx.accounts.payer;
    let token_program = &ctx.accounts.token_program;
    subscription_account.plan_account = plan_account.key();
    subscription_account.payer_token_account = payer_token_account.key();
    subscription_account.owner = payer.key();
    subscription_account.state = SubscriptionState::Active;
    subscription_account.next_term_date =
        Clock::get()?.unix_timestamp + (plan_account.term_in_seconds as i64);
    plan_account.active_subscriptions += 1;
    let approve_accounts = Approve {
        delegate: subscription_account.to_account_info().clone(),
        to: payer_token_account.to_account_info().clone(),
        authority: payer.to_account_info().clone(),
    };
    approve(
        CpiContext::new(token_program.to_account_info().clone(), approve_accounts),
        data.delegation_amount,
    )?;
    let transfer_accounts = Transfer {
        from: payer_token_account.to_account_info().clone(),
        to: plan_token_account.to_account_info().clone(),
        authority: payer.to_account_info().clone(),
    };
    transfer(
        CpiContext::new(token_program.to_account_info().clone(), transfer_accounts),
        plan_account.price,
    )?;
    Ok(())
}





#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq)]
pub enum SubscriptionState {
    #[default]
    Active,
    PendingCancellation,
    PastDue,
}

#[account]
pub struct Subscription {
    pub plan_account: Pubkey,        // 32
    pub payer_token_account: Pubkey, // 32
    pub next_term_date: i64,         // 8
    pub owner: Pubkey,               // 32
    pub state: SubscriptionState,    // 1 + 10 = 11
}

#[derive(Accounts)]
pub struct CreateSubscriptionParams<'info> {
    #[account(
        init, 
        payer = payer, 
        space =  8 + 32 + 32 + 32 + 8 + 11,
        seeds = [b"subscription".as_ref(), payer.key().as_ref(), plan_account.key().as_ref()],
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
        constraint = payer_token_account.mint == plan_account.token_mint,
        constraint = payer_token_account.owner == payer.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = plan_token_account.mint == plan_account.token_mint,
        constraint = plan_token_account.owner == plan_account.key(),
    )]
    pub plan_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct CreateSubscriptionData {
    pub delegation_amount: u64,
}
