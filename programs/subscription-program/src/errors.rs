use anchor_lang::prelude::*;

#[error_code]
pub enum SubscriptionErrors {
    #[msg("Subscription is not ready to be credited")]
    SubscriptionNotReady,
}
