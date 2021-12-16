// use crate::*;
// use near_sdk::{
//     assert_one_yocto,
//     borsh::{self, BorshDeserialize, BorshSerialize},
//     json_types::U128,
//     near_bindgen,
//     serde::{Deserialize, Serialize},
//     AccountId,
// };

// impl Payouts for Contract {
//     #[allow(unused_variables)]
//     fn nft_payout(&self, token_id: String, balance: U128, max_len_payout: Option<u32>) -> Payout {
//         let owner_id = self
//             .tokens
//             .owner_by_id
//             .get(&token_id)
//             .expect("No such token_id");
//         self.royalties
//             .get()
//             .map_or(Payout::default(), |r| r.create_payout(balance.0, &owner_id))
//     }

//     #[payable]
//     fn nft_transfer_payout(
//         &mut self,
//         receiver_id: AccountId,
//         token_id: String,
//         approval_id: Option<u64>,
//         memo: Option<String>,
//         balance: U128,
//         max_len_payout: Option<u32>,
//     ) -> Payout {
//         assert_one_yocto();
//         let payout = self.nft_payout(token_id.clone(), balance, max_len_payout);
//         self.nft_transfer(
//             receiver_id,
//             token_id,
//             approval_id,
//             memo,
//         );
//         payout
//     }
// }
