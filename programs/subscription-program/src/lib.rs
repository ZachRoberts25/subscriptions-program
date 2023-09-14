use anchor_lang::prelude::*;
pub mod state;
use anchor_spl::token::{approve, transfer, Approve, Transfer};
pub use state::*;
pub mod errors;
pub use errors::SubscriptionErrors;

declare_id!("6beY4Na32mmSym2oibGVBfM43B69CzrY3VQ7Uvu77LaN");

#[program]
pub mod subscription_program {

    use super::*;

    pub fn create_plan(ctx: Context<CreatePlanParams>, data: CreatePlanData) -> Result<()> {
        let plan_account = &mut ctx.accounts.plan_account;
        let settlement_token_account = &mut ctx.accounts.settlement_token_account;
        plan_account.code = data.code;
        plan_account.creator = *ctx.accounts.payer.key;
        plan_account.settlement_token_account = settlement_token_account.key();
        plan_account.price = data.price;
        plan_account.token_mint = settlement_token_account.mint;
        plan_account.term = data.term;
        Ok(())
    }

    pub fn create_subscription(
        ctx: Context<CreateSubscriptionParams>,
        data: CreateSubscriptionData,
    ) -> Result<()> {
        let plan_account = &mut ctx.accounts.plan_account;
        let subscription_account = &mut ctx.accounts.subscription_account;
        let payer_token_account = &mut ctx.accounts.payer_token_account;
        let settlement_token_account = &mut ctx.accounts.settlement_token_account;
        let pda_account = &mut ctx.accounts.pda_account;
        let payer = &mut ctx.accounts.payer;
        let token_program = &ctx.accounts.token_program;
        subscription_account.plan_account = plan_account.key();
        subscription_account.payer_token_account = payer_token_account.key();
        subscription_account.owner = payer.key();
        let current = Clock::get()?.unix_timestamp;
        if plan_account.term == Term::OneWeek {
            subscription_account.next_term_date = current + 604800;
        } else if plan_account.term == Term::OneSecond {
            subscription_account.next_term_date = current + 1;
        } else if plan_account.term == Term::ThirtyDays {
            subscription_account.next_term_date = current + 2592000;
        } else if plan_account.term == Term::OneYear {
            subscription_account.next_term_date = current + 31536000;
        }
        let approve_accounts = Approve {
            delegate: pda_account.to_account_info().clone(),
            to: payer_token_account.to_account_info().clone(),
            authority: payer.to_account_info().clone(),
        };
        approve(
            CpiContext::new(token_program.to_account_info().clone(), approve_accounts),
            data.delegation_amount,
        )?;
        let transfer_accounts = Transfer {
            from: payer_token_account.to_account_info().clone(),
            to: settlement_token_account.to_account_info().clone(),
            authority: payer.to_account_info().clone(),
        };
        transfer(
            CpiContext::new(token_program.to_account_info().clone(), transfer_accounts),
            plan_account.price,
        )?;
        Ok(())
    }

    pub fn charge_subscription(ctx: Context<ChargeSubscriptionParams>) -> Result<()> {
        let plan_account = &mut ctx.accounts.plan_account;
        let subscription_account = &mut ctx.accounts.subscription_account;
        let pda_account = &mut ctx.accounts.pda_account;
        let settlement_token_account = &mut ctx.accounts.settlement_token_account;
        let subscriber_token_account = &mut ctx.accounts.subscriber_token_account;

        let current = Clock::get()?.unix_timestamp;
        if current < subscription_account.next_term_date {
            return Err(SubscriptionErrors::SubscriptionNotReady.into());
        }
        let transfer_accounts = Transfer {
            from: subscriber_token_account.to_account_info().clone(),
            to: settlement_token_account.to_account_info().clone(),
            authority: pda_account.to_account_info().clone(),
        };

        let (_pda, bump) = Pubkey::find_program_address(
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
                    &[bump],
                ]],
            ),
            plan_account.price,
        )?;
        Ok(())
    }
}
