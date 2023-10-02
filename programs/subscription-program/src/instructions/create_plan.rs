use anchor_lang::prelude::*;
use anchor_spl::{token::{Token, TokenAccount, Mint}, associated_token::AssociatedToken};


pub fn handle_create_plan(ctx: Context<CreatePlanParams>, data: CreatePlanData) -> Result<()> {
    let plan_account = &mut ctx.accounts.plan_account;
    let plan_token_account = &mut ctx.accounts.plan_token_account;
    plan_account.code = data.code;
    plan_account.owner = *ctx.accounts.payer.key;
    plan_account.price = data.price;
    plan_account.token_mint = plan_token_account.mint;
    plan_account.term_in_seconds = data.term_in_seconds;
    plan_account.active_subscriptions = 0;
    Ok(())
}



#[derive(Accounts)]
#[instruction(code: String)]
pub struct CreatePlanParams<'info> {
    #[account(
        init, 
        payer = payer, 
        space = 8 + 36 + 32 + 8 + 32 + 8 + 4, 
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
    pub term_in_seconds: u64,
}


#[account]
pub struct Plan {
    pub code: String,                   // 4 + 32 = 36
    pub owner: Pubkey,                  // 32
    pub price: u64,                     // 8
    pub token_mint: Pubkey,             // 32
    pub term_in_seconds: u64,           // 8
    pub active_subscriptions: u32,      // 4
}
