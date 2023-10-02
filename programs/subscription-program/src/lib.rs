use anchor_lang::prelude::*;
pub mod errors;
pub use errors::SubscriptionErrors;
pub mod instructions;
use instructions::{
    cancel_subscription::*, charge_subscription::*, close_subscription::*, create_plan::*,
    create_subscription::*, uncancel_subscription::*,
};

declare_id!("6qMvvisbUX3Co1sZa7DkyCXF8FcsTjzKSQHcaDoqSLbw");

#[program]
pub mod subscription_program {

    use crate::instructions::create_plan::handle_create_plan;

    use super::*;

    pub fn create_plan(ctx: Context<CreatePlanParams>, data: CreatePlanData) -> Result<()> {
        handle_create_plan(ctx, data)
    }

    pub fn create_subscription(
        ctx: Context<CreateSubscriptionParams>,
        data: CreateSubscriptionData,
    ) -> Result<()> {
        handle_create_subscription(ctx, data)
    }

    pub fn charge_subscription(ctx: Context<ChargeSubscriptionParams>) -> Result<()> {
        handle_charge_subscription(ctx)
    }

    pub fn cancel_subscription(ctx: Context<CancelSubscriptionParams>) -> Result<()> {
        handle_cancel_subscription(ctx)
    }

    pub fn uncancel_subscription(ctx: Context<UncancelSubscriptionParams>) -> Result<()> {
        handle_uncancel_subscription(ctx)
    }

    pub fn close_subscription(ctx: Context<CloseSubscriptionParams>) -> Result<()> {
        handle_close_subscription(ctx)
    }
}
