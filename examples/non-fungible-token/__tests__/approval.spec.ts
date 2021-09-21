import path from 'path';
import { NEAR, Gas } from 'near-units';
import type { Token } from './utils';
import { createRunner, TOKEN_ID } from './utils';

const runner = createRunner(async ({ root, nft }) => ({
  tokenReceiver: await root.createAndDeploy(
    'token-receiver',
    path.join(__dirname, '..', 'res', 'token_receiver.wasm'),
    {
      method: 'new',
      args: { non_fungible_token_account_id: nft }
    }
  ),
  approvalReceiver: await root.createAndDeploy(
    'approval-receiver',
    path.join(__dirname, '..', 'res', 'approval_receiver.wasm'),
    {
      method: 'new',
      args: { non_fungible_token_account_id: nft }
    }
  ),
}));

describe('nft_approve', () => {
  runner.test('has expected approval_id logic', async ({ root, alice, nft, tokenReceiver }) => {
    await root.call(
      nft,
      'nft_approve',
      {
        token_id: TOKEN_ID,
        account_id: alice,
      },
      {
        attachedDeposit: NEAR.parse('270 microNEAR')
      }
    );

    // check nft_is_approved, don't provide approval_id
    const aliceApproved = await nft.view('nft_is_approved', {
      token_id: TOKEN_ID,
      approved_account_id: alice,
    }) as boolean;
    expect(aliceApproved).toBe(true);

    // check nft_is_approved, with approval_id=1
    const aliceApprovalIs1 = await nft.view('nft_is_approved', {
      token_id: TOKEN_ID,
      approved_account_id: alice,
      approval_id: 1,
    }) as boolean;
    expect(aliceApprovalIs1).toBe(true);

    // check nft_is_approved, with approval_id=2
    let aliceApprovalIs2 = await nft.view('nft_is_approved', {
      token_id: TOKEN_ID,
      approved_account_id: alice,
      approval_id: 2,
    }) as boolean;
    expect(aliceApprovalIs2).toBe(false);

    // alternatively, one could check the data returned by nft_token
    const token = await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(token.approved_account_ids).toEqual({ [alice.accountId]: 1 });

    // root approves alice again, which changes the approval_id and doesn't require as much deposit
    await root.call(
      nft,
      'nft_approve',
      {
        token_id: TOKEN_ID,
        account_id: alice,
      },
      {
        attachedDeposit: NEAR.parse('1 yoctoNEAR'),
      }
    );

    (aliceApprovalIs2 = await nft.view('nft_is_approved', {
      token_id: TOKEN_ID,
      approved_account_id: alice,
      approval_id: 2,
    }) as boolean);
    expect(aliceApprovalIs2).toBe(true);

    // approving another account gives different approval_id
    await root.call(
      nft,
      'nft_approve',
      {
        token_id: TOKEN_ID,
        account_id: tokenReceiver,
      },
      {
        // note that tokenReceiver's account name is longer, and so it takes
        // more bytes to store and therefore requires a larger deposit!
        attachedDeposit: NEAR.parse('360 microNEAR')
      }
    );

    let tokenReceiverApprovalIs3 = await nft.view('nft_is_approved', {
      token_id: TOKEN_ID,
      approved_account_id: alice,
      approval_id: 2,
    }) as boolean;
    expect(tokenReceiverApprovalIs3).toBe(true);
  });
});