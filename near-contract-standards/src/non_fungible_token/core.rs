use near_sdk::json_types::ValidAccountId;
use near_sdk::json_types::U128;
use near_sdk::Promise;

pub trait NonFungibleTokenCore {
    /// Simple transfer. Transfer a given `token_id` from current owner to
    /// `receiver_id`.
    ///
    /// Requirements
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security purposes
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * `approval_id` is for use with Approval Management,
    ///   see https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    /// * TODO: needed? Both accounts must be registered with the contract for transfer to
    ///   succeed. See see https://nomicon.io/Standards/StorageManagement.html
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token
    /// * `token_id`: the token to transfer
    /// * `approval_id`: expected approval ID. A number smaller than
    ///    2^53, and therefore representable as JSON. See Approval Management
    ///    standard for full explanation.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: String,
        approval_id: Option<U64>,
        memo: Option<String>,
    );

    /// Transfer token and call a method on a receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the NFT
    /// contract at the method `nft_resolve_transfer`.
    ///
    /// You can think of this as being similar to attaching native NEAR tokens to a
    /// function call. It allows you to attach any Non-Fungible Token in a call to a
    /// receiver contract.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security
    ///   purposes
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * The receiving contract must implement `ft_on_transfer` according to the
    ///   standard. If it does not, FT contract's `ft_resolve_transfer` MUST deal
    ///   with the resulting failed cross-contract call and roll back the transfer.
    /// * Contract MUST implement the behavior described in `ft_resolve_transfer`
    /// * `approval_id` is for use with Approval Management extension, see
    ///   that document for full explanation.
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_id`: the token to send.
    /// * `approval_id`: expected approval ID. A number smaller than
    ///    2^53, and therefore representable as JSON. See Approval Management
    ///    standard for full explanation.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer.
    /// * `msg`: specifies information needed by the receiving contract in
    ///    order to properly handle the transfer. Can indicate both a function to
    ///    call and the parameters to pass to that function.
    fn nft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<U64>,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    /// Returns the token with the given `token_id` or `null` if no such token.
    fn nft_token(token_id: U128) -> Option<JsonToken>;
}
