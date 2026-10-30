use giftcard_core::{step, Code, Event};
use proptest::prelude::*;

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![Just(Event::Sale), Just(Event::Redemption), Just(Event::Expiry)]
}

proptest! {
    #[test]
    fn redeemed_at_most_once(events in prop::collection::vec(event(), 0..50)) {
        let mut code = Code::Available;
        let mut redemptions = 0;
        for e in events {
            let (next, _) = step(code, e);
            if next == Code::Redeemed && code != Code::Redeemed {
                redemptions += 1;
            }
            code = next;
        }
        prop_assert!(redemptions <= 1);
    }
}
