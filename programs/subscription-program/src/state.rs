use anchor_lang::prelude::*;
use anchor_spl::{token::{Token, TokenAccount, Mint}, associated_token::AssociatedToken};
use solana_program::pubkey;


#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq, Copy)]
pub enum Term {
    // for testing purposes;
    OneSecond,
    ThirtySeconds,
    OneWeek,
    #[default]
    ThirtyDays,
    OneYear,
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug, PartialEq)]
pub enum SubscriptionState {
    #[default]
    Active,
    PendingCancellation,
    PastDue,
}

#[account]
pub struct Plan {
    pub code: String,                     // 4 + 32 = 36
    pub owner: Pubkey,                  // 32
    pub price: u64,                       // 8
    pub token_mint: Pubkey,               // 32
    pub term: Term,                       // 1 + 10 = 11
    pub active_subscriptions: u32,        // 4

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

#[derive(Accounts)]
#[instruction(code: String)]
pub struct CreatePlanParams<'info> {
    #[account(
        init, 
        payer = payer, 
        space = 8 + 36 + 84 + 32 + 32 + 8 + 32 + 11 + 10, 
        seeds = [b"plan".as_ref(), payer.key().as_ref(), code.as_ref()],
        bump
    )]
    pub plan_account: Account<'info, Plan>,
    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = plan_account,
    )]
    pub plan_token_account: Account<'info, TokenAccount>,
    #[account()]
    pub mint_account: Account<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(mut)]
    pub payer: Signer<'info>,
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


#[derive(Accounts)]
pub struct CancelSubscriptionParams<'info> {
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        constraint = subscription_account.owner == payer.key(),
        constraint = subscription_account.state == SubscriptionState::Active,
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


#[derive(Accounts)]
pub struct CloseSubscriptionParams<'info> {
    #[account(
        mut,
        seeds = [b"subscription".as_ref(), subscription_account.owner.key().as_ref(), plan_account.key().as_ref()],
        constraint = subscription_account.owner == payer.key(),
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
