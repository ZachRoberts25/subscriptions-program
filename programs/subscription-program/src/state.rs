use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq)]
pub enum Term {
    // for testing purposes;
    OneSecond,
    OneWeek,
    #[default]
    ThirtyDays,
    OneYear,
}

#[account]
pub struct Plan {
    pub code: String,                     // 4 + 32 = 36
    pub creator: Pubkey,                  // 32
    pub settlement_token_account: Pubkey, // 32
    pub price: u64,                       // 8
    pub token_mint: Pubkey,               // 32
    pub term: Term,                       // 1 + 10 = 11
}

#[account]
pub struct Subscription {
    pub plan_account: Pubkey,        // 32
    pub payer_token_account: Pubkey, // 32
    pub next_term_date: i64,         // 8
    pub owner: Pubkey,               // 32
}

#[derive(Accounts)]
pub struct CreateSubscriptionParams<'info> {
    #[account(init, payer = payer, space =  8 + 32 + 32 + 32 + 8)]
    pub subscription_account: Account<'info, Subscription>,
    #[account()]
    pub plan_account: Account<'info, Plan>,
    #[account(
        mut,
        constraint = payer_token_account.mint == plan_account.token_mint,
        constraint = payer_token_account.owner == payer.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), payer.key().as_ref(), plan_account.key().as_ref()],
        bump,
    )]
    pub pda_account: SystemAccount<'info>,
    #[account(
        mut,
        constraint = settlement_token_account.key().as_ref() == plan_account.settlement_token_account.as_ref()
    )]
    pub settlement_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct CreateSubscriptionData {
    pub delegation_amount: u64,
}

#[derive(Accounts)]
pub struct CreatePlanParams<'info> {
    #[account(init, payer = payer, space = 8 + 36 + 84 + 32 + 32 + 8 + 32 + 11)]
    pub plan_account: Account<'info, Plan>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub settlement_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct CreatePlanData {
    pub code: String,
    pub price: u64,
    pub term: Term,
}

#[derive(Accounts)]
pub struct ChargeSubscriptionParams<'info> {
    #[account(mut)]
    pub subscription_account: Account<'info, Subscription>,
    #[account()]
    pub plan_account: Account<'info, Plan>,
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        bump,
    )]
    pub pda_account: SystemAccount<'info>,
    #[account(
        mut,
        constraint = settlement_token_account.key().as_ref() == plan_account.settlement_token_account.as_ref()
    )]
    pub settlement_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = subscriber_token_account.mint == plan_account.token_mint,
        constraint = subscriber_token_account.owner == subscription_account.owner.key(),
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
