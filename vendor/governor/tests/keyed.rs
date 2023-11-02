use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use nonzero_ext::nonzero;

#[test]
fn default_keyed() {
    let limiter: DefaultKeyedRateLimiter<u32> =
        RateLimiter::keyed(Quota::per_second(nonzero!(20u32)));
    assert_eq!(Ok(()), limiter.check_key(&1));
}
