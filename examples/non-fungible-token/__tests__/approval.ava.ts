import { NEAR } from 'near-workspaces-ava';
import type { Token } from './utils';
import { createWorkspace, TOKEN_ID } from './utils';

const workspace = createWorkspace(async ({ root, nft }) => ({
  tokenReceiver: await root.createAndDeploy(
    'token-receiver',
    'non-fungible-token/res/token_receiver.wasm',
    {
      method: 'new',
      args: { non_fungible_token_account_id: nft }
    }
  ),
  approvalReceiver: await root.createAndDeploy(
    'approval-receiver',
    'non-fungible-token/res/approval_receiver.wasm',
    {
      method: 'new',
      args: { non_fungible_token_account_id: nft }
    }
  ),
}));

workspace.test('nft_approve has expected approval_id logic', async (t, { root, alice, nft, tokenReceiver }) => {
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('270μN')
  });

  // check nft_is_approved, don't provide approval_id
  const aliceApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
  }) as boolean;
  t.is(aliceApproved, true);

  // check nft_is_approved, with approval_id=1
  const aliceApprovalIs1 = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
    approval_id: 1,
  }) as boolean;
  t.is(aliceApprovalIs1, true);

  // check nft_is_approved, with approval_id=2
  let aliceApprovalIs2 = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
    approval_id: 2,
  }) as boolean;
  t.is(aliceApprovalIs2, false);

  // alternatively, one could check the data returned by nft_token
  const token = await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.deepEqual(token.approved_account_ids, { [alice.accountId]: 1 });

  // root approves alice again, which changes the approval_id and doesn't require as much deposit
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('1 yoctoNEAR'),
  });

  (aliceApprovalIs2 = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
    approval_id: 2,
  }) as boolean);
  t.is(aliceApprovalIs2, true);

  // approving another account gives different approval_id
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: tokenReceiver,
  }, {
    // note that tokenReceiver's account name is longer, and so it takes
    // more bytes to store and therefore requires a larger deposit!
    attachedDeposit: NEAR.parse('360μN')
  });

  let tokenReceiverApprovalIs3 = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
    approval_id: 2,
  }) as boolean;
  t.is(tokenReceiverApprovalIs3, true);
});

workspace.test('nft_approve creates cross-contract call if given `msg`', async (t, { root, nft, approvalReceiver }) => {
  let tx = await root.call_raw(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: approvalReceiver,
    msg: 'return-now',
  }, {
    attachedDeposit: NEAR.parse('390μN')
  });

  // Make sure all cross-contract calls worked as expected!
  // This is needed because  gracefully ignores cross-contract call failures,
  // so the transaction can pass even if cross-contract calls fail.
  t.is(tx.errors.length, 0);
  t.regex(tx.logs[0], /approval_id=1/);
  t.is(tx.parseResult(), 'cool');

  // Approve again; will set different approval_id (ignored by this approvalReceiver implementation).
  // The approval_receiver implementation will return given `msg` after subsequent promise call,
  // if given something other than "return-now".
  const msg = 'hahaha';
  tx = await root.call_raw(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: approvalReceiver,
    msg,
  }, {
    attachedDeposit: NEAR.parse('1 yN')
  });
  t.is(tx.errors.length, 0);
  t.regex(tx.logs[0], /approval_id=2/);
  t.is(tx.parseResult(), msg);
});

workspace.test('nft_approve allows approved account to transfer token', async (t, { root, nft, alice }) => {
  // root approves alice
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('270μN')
  });

  // alice sends to self
  await alice.call_raw(nft, 'nft_transfer', {
    receiver_id: alice,
    token_id: TOKEN_ID,
    memo: 'mine now! 😈',
  }, {
    attachedDeposit: '1'
  });

  // token now owned by alice
  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, alice.accountId);
})

workspace.test('nft_revoke', async (t, { root, nft, alice, tokenReceiver }) => {
  // root approves alice
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('270μN')
  });

  // root approves tokenReceiver
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: tokenReceiver,
  }, {
    attachedDeposit: NEAR.parse('390μN')
  });

  // root revokes alice
  await root.call(nft, 'nft_revoke', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('1yN')
  });

  // alice is revoked...
  let aliceApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
  }) as boolean;
  t.is(aliceApproved, false);

  // ...but token_receiver is still approved
  let tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  t.is(tokenReceiverApproved, true);

  // root revokes tokenReceiver
  await root.call(nft, 'nft_revoke', {
    token_id: TOKEN_ID,
    account_id: tokenReceiver,
  }, {
    attachedDeposit: NEAR.parse('1yN')
  });

  // alice is still revoked...
  aliceApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
  }) as boolean;
  t.is(aliceApproved, false);

  // ...and now so is tokenReceiver
  tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  t.is(tokenReceiverApproved, false);
});

workspace.test('nft_revoke_all', async (t, { root, nft, alice, tokenReceiver }) => {
  // root approves alice
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: alice,
  }, {
    attachedDeposit: NEAR.parse('270μN')
  });

  // root approves tokenReceiver
  await root.call(nft, 'nft_approve', {
    token_id: TOKEN_ID,
    account_id: tokenReceiver,
  }, {
    attachedDeposit: NEAR.parse('390μN')
  });

  // root revokes all
  await root.call(nft, 'nft_revoke_all', {
    token_id: TOKEN_ID,
  }, {
    attachedDeposit: NEAR.parse('1yN')
  });

  // alice is revoked...
  const aliceApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: alice,
  }) as boolean;
  t.is(aliceApproved, false);

  // ...and so is tokenReceiver
  const tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  t.is(tokenReceiverApproved, false);
});
