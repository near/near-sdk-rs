use near_bindgen::{near_bindgen, ENV};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type AccountId = String;

/// Token interface.
/// transfer(receiver, amount)
///     Just transfers given amount to receiver from senders(!) account.
/// payTo(receiver, amount, methodName, arguments);
///     payTo transfers money to payTo + passes "amount" as last extra argument to the given ones.
///     fails if amount is already in arguments.
///     e.g. `receiver.methodName([arguments.., amount])`

/// Interface for an auction of one token for another token.
trait DutchAuction {
    /// Start auction, sets all the required parameters.
    /// @auction_token: contract name of token to sell. Use `system` to sell `NEAR` tokens.
    /// @exchange_token: contract name of token that we sell for.
    fn start_auction(
        &mut self,
        auction_token: AccountId,
        exchange_token: AccountId,
        auction_length: u64,
        start_price: u128,
        end_price: u128,
        amount: u128,
    );
    /// Returns auction price at current block. If auction didn't start, returns 0.
    fn price_at_block(&self) -> u128;
    /// Bid attached amount to sender. Should be used in `payTo`
    fn bid(&mut self, amount: u128);
    /// Claim tokens for bids after auction.
    fn claim(&mut self);
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
enum AuctionStage {
    AuctionDeployed,
    AuctionStarted,
    AuctionEnded,
}

impl Default for AuctionStage {
    fn default() -> Self {
        AuctionStage::AuctionDeployed
    }
}

#[near_bindgen]
#[derive(Serialize, Deserialize, Default)]
pub struct NearDutchAuction {
    owner: AccountId,
    auction_stage: AuctionStage,
    start_block: u64,
    end_block: u64,
    amount: u128,
    total_received: u128,
    auction_token: AccountId,
    exchange_token: AccountId,
    start_price: u128,
    end_price: u128,

    final_price: u128,
    bids: HashMap<AccountId, u128>,
}

#[near_bindgen]
impl DutchAuction for NearDutchAuction {
    fn start_auction(
        &mut self,
        auction_token: AccountId,
        exchange_token: AccountId,
        auction_length: u64,
        start_price: u128,
        end_price: u128,
        amount: u128,
    ) {
        let originator = String::from_utf8(ENV.originator_id()).unwrap();
        assert_eq!(self.auction_stage, AuctionStage::AuctionDeployed);
        assert_eq!(originator, auction_token);
        self.owner = originator;
        self.start_block = ENV.block_index();
        self.end_block = self.start_block + auction_length;
        self.auction_token = auction_token;
        self.exchange_token = exchange_token;
        self.amount = amount;
        self.total_received = 0;
        self.start_price = start_price;
        self.end_price = end_price;
        self.auction_stage = AuctionStage::AuctionStarted;
    }

    fn price_at_block(&self) -> u128 {
        match self.auction_stage {
            AuctionStage::AuctionDeployed => 0,
            AuctionStage::AuctionStarted => self.calc_price(ENV.block_index()),
            AuctionStage::AuctionEnded => self.final_price,
        }
    }

    fn bid(&mut self, amount: u128) {
        let mut amount = amount;
        let originator = String::from_utf8(ENV.originator_id()).unwrap();
        assert_eq!(originator, self.exchange_token);

        // The amount left to deposit is 0 after the auction has finished.
        let max_left = if ENV.block_index() > self.end_block {
            0
        } else {
            self.amount * self.calc_price(ENV.block_index()) - self.total_received
        };

        if amount > max_left {
            ENV.promise_create(
                self.exchange_token.as_bytes(),
                "transfer".as_bytes(),
                format!(
                    "{{\"receiver\": \"{}\", \"amount\": \"{}\"}}",
                    originator,
                    amount - max_left
                )
                .as_bytes(),
                1,
            );
            amount = max_left;
        }

        if amount > 0 {
            *self.bids.entry(originator).or_insert(0) += amount;
            self.total_received += amount;
        }

        // If amount is equal max_left, finalize auction.
        if amount == max_left {
            self.finalize_auction();
        }
    }

    fn claim(&mut self) {
        assert_eq!(self.auction_stage, AuctionStage::AuctionEnded);
        let receiver = String::from_utf8(ENV.originator_id()).unwrap();
        let bid_amount = self.bids.remove(&receiver).unwrap_or_default();
        let amount = bid_amount / self.final_price;
        ENV.promise_create(
            self.auction_token.as_bytes(),
            "transfer".as_bytes(),
            format!("{{\"receiver\": \"{}\", \"amount\": \"{}\"}}", receiver, amount).as_bytes(),
            1,
        );
    }
}

impl NearDutchAuction {
    fn calc_price(&self, block_index: u64) -> u128 {
        if block_index >= self.end_block {
            return self.end_price;
        }
        let price_factor =
            (self.start_price - self.end_price) / ((self.end_block - self.start_block) as u128);
        self.end_price + price_factor * ((self.end_block - block_index) as u128)
    }

    fn finalize_auction(&mut self) {
        self.auction_stage = AuctionStage::AuctionEnded;
        self.final_price = self.calc_price(ENV.block_index());
        let sold_tokens = self.total_received / self.final_price;
        // Send owner the rest of the tokens that were not sold.
        ENV.promise_create(
            self.auction_token.as_bytes(),
            "transfer".as_bytes(),
            format!(
                "{{\"receiver\": \"{}\", \"amount\": \"{}\"}}",
                self.owner,
                self.amount - sold_tokens
            )
            .as_bytes(),
            1,
        );
        self.amount -= sold_tokens;
    }
}

#[cfg(feature = "env_test")]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::MockedEnvironment;
    use near_bindgen::ENV;

    /// Start auction from 0-1000 blocks, selling 10_000 NEAR for ETH, start price is 10_000, lowest price 100.
    #[test]
    fn test_auction_bid() {
        ENV.set(Box::new(MockedEnvironment::new()));
        ENV.as_mock().set_block_index(0);
        ENV.as_mock().set_originator_id("alice".as_bytes().to_vec());
        let mut auction = NearDutchAuction::default();
        auction.start_auction("system".to_string(), "eth".to_string(), 100, 10_000, 100, 10_000);
        assert_eq!(ENV.as_mock().get_promise_create(), vec![]);
        assert_eq!(auction.price_at_block(), 10_000);
        ENV.as_mock().set_block_index(100);
        assert_eq!(auction.price_at_block(), 100);
        ENV.as_mock().set_block_index(1000);
        assert_eq!(auction.price_at_block(), 100);

        ENV.as_mock().set_block_index(5);
        ENV.as_mock().set_originator_id("bob".as_bytes().to_vec());
        auction.bid(1_000);
        assert_eq!(ENV.as_mock().get_promise_create(), vec![]);

        // Too late, auction is finished. This will finalize it and return peter ETH.
        ENV.as_mock().set_block_index(101);
        ENV.as_mock().set_originator_id("peter".as_bytes().to_vec());
        auction.bid(1_000);
        assert_eq!(ENV.as_mock().get_promise_create(), vec![]);

        ENV.as_mock().set_block_index(102);
        ENV.as_mock().set_originator_id("bob".as_bytes().to_vec());
        auction.claim();
        // Bob receives what he bought.
        assert_eq!(ENV.as_mock().get_promise_create(), vec![]);
        // Final price is the same after we are done.
        ENV.as_mock().set_block_index(1000);
        ENV.as_mock().set_originator_id(vec![]);
        assert_eq!(auction.price_at_block(), 100);
    }
}
