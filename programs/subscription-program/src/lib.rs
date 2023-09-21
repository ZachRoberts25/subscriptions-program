use anchor_lang::prelude::*;
pub mod state;
use anchor_spl::token::{approve, transfer, Approve, Transfer};
pub use state::*;
pub mod errors;
pub use errors::SubscriptionErrors;

declare_id!("6qMvvisbUX3Co1sZa7DkyCXF8FcsTjzKSQHcaDoqSLbw");

pub fn term_to_seconds(term: Term) -> i64 {
    if term == Term::OneSecond {
        return 1;
    } else if term == Term::ThirtySeconds {
        return 30;
    } else if term == Term::OneWeek {
        return 604800;
    } else if term == Term::ThirtyDays {
        return 2592000;
    } else if term == Term::OneYear {
        return 31536000;
    } else {
        return 0;
    }
}

#[program]
pub mod subscription_program {

    use super::*;

    pub fn create_plan(ctx: Context<CreatePlanParams>, data: CreatePlanData) -> Result<()> {
        let plan_account = &mut ctx.accounts.plan_account;
        let plan_token_account = &mut ctx.accounts.plan_token_account;
        plan_account.code = data.code;
        plan_account.owner = *ctx.accounts.payer.key;
        plan_account.price = data.price;
        plan_account.token_mint = plan_token_account.mint;
        plan_account.term = data.term;
        plan_account.active_subscriptions = 0;
        Ok(())
    }

    pub fn create_subscription(
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
            Clock::get()?.unix_timestamp + term_to_seconds(plan_account.term);
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

    pub fn charge_subscription(ctx: Context<ChargeSubscriptionParams>) -> Result<()> {
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
        subscription_account.next_term_date += term_to_seconds(plan_account.term);
        Ok(())
    }

    pub fn cancel_subscription(ctx: Context<CancelSubscriptionParams>) -> Result<()> {
        // cancel will cancel the subscription but let it finish out the current term;
        let subscription_account = &mut ctx.accounts.subscription_account;
        subscription_account.state = SubscriptionState::PendingCancellation;
        Ok(())
    }

    pub fn uncancel_subscription(ctx: Context<UncancelSubscriptionParams>) -> Result<()> {
        // cancel will cancel the subscription but let it finish out the current term;
        let subscription_account = &mut ctx.accounts.subscription_account;
        subscription_account.state = SubscriptionState::Active;
        Ok(())
    }

    pub fn close_subscription(ctx: Context<CloseSubscriptionParams>) -> Result<()> {
        // close immediately, closes the subscription account and refunds the user for the remaining time;
        let subscription_account = &mut ctx.accounts.subscription_account;
        let plan_account = &mut ctx.accounts.plan_account;
        let plan_token_account = &mut ctx.accounts.plan_token_account;
        let payer_token_account = &mut ctx.accounts.payer_token_account;
        let plan_owner_token_account = &ctx.accounts.plan_owner_token_account;
        let deployer_token_account = &ctx.accounts.deployer_token_account;
        let token_program = &ctx.accounts.token_program;
        let current = Clock::get()?.unix_timestamp;
        plan_account.active_subscriptions -= 1;
        // the subscription end date is in the future so the user needs a refund for the remaining time;
        if current < subscription_account.next_term_date {
            let term_seconds = term_to_seconds(plan_account.term);
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
        Ok(())
    }
}
