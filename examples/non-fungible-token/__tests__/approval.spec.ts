import path from 'path';
import { NEAR } from 'near-units';
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
    expect(aliceApprovalIs2).toBe(true);

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
    expect(tokenReceiverApprovalIs3).toBe(true);
  });

  runner.test('creates cross-contract call if given `msg`', async ({ root, nft, approvalReceiver }) => {
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
    expect(tx.errors.length).toBe(0);
    expect(tx.logs[0]).toMatch('approval_id=1');
    expect(tx.parseResult()).toBe('cool');

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
    expect(tx.errors.length).toBe(0);
    expect(tx.logs[0]).toMatch('approval_id=2');
    expect(tx.parseResult()).toBe(msg);
  });

  runner.test('allows approved account to transfer token', async ({ root, nft, alice }) => {
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
    expect(owner_id).toBe(alice.accountId);
  })
});

runner.test('nft_revoke', async ({ root, nft, alice, tokenReceiver }) => {
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
  expect(aliceApproved).toBe(false);

  // ...but token_receiver is still approved
  let tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  expect(tokenReceiverApproved).toBe(true);

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
  expect(aliceApproved).toBe(false);

  // ...and now so is tokenReceiver
  tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  expect(tokenReceiverApproved).toBe(false);
});

runner.test('nft_revoke_all', async ({ root, nft, alice, tokenReceiver }) => {
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
  expect(aliceApproved).toBe(false);

  // ...and so is tokenReceiver
  const tokenReceiverApproved = await nft.view('nft_is_approved', {
    token_id: TOKEN_ID,
    approved_account_id: tokenReceiver,
  }) as boolean;
  expect(tokenReceiverApproved).toBe(false);
});