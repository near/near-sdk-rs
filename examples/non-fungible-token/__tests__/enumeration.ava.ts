import { NEAR } from 'near-workspaces-ava';
import type { Token } from './utils';
import { createWorkspace, TOKEN_ID } from './utils';

const workspace = createWorkspace(async ({ root, nft }) => {
  await root.call(nft, 'nft_mint', {
    token_id: '1',
    token_owner_id: root,
    token_metadata: {
      title: 'Black as the Night',
      description: 'In charcoal',
      copies: 1,
    },
  }, {
    attachedDeposit: NEAR.parse('4.26mN'),
  });
  await root.call(nft, 'nft_mint', {
    token_id: '2',
    token_owner_id: root,
    token_metadata: {
      title: 'Hamakua',
      description: 'Vintage recording',
      copies: 1,
    },
  }, {
    attachedDeposit: NEAR.parse('4.21mN'),
  });
  await root.call(nft, 'nft_mint', {
    token_id: '3',
    token_owner_id: root,
    token_metadata: {
      title: 'Aloha ke akua',
      description: 'Original with piano',
      copies: 1,
    },
  }, {
    attachedDeposit: NEAR.parse('4.29mN'),
  });
});

workspace.test('nft_supply_for_owner', async (t, { root, alice, nft }) => {
  t.is(
    await nft.view('nft_supply_for_owner', { account_id: alice }),
    '0'
  );
  t.is(
    await nft.view('nft_supply_for_owner', { account_id: root }),
    '4'
  );

  await root.call(nft, 'nft_transfer', {
    token_id: TOKEN_ID,
    receiver_id: alice,
  }, {
    attachedDeposit: '1'
  });

  t.is(
    await nft.view('nft_supply_for_owner', { account_id: alice }),
    '1'
  );
  t.is(
    await nft.view('nft_supply_for_owner', { account_id: root }),
    '3'
  );
});

workspace.test('nft_total_supply', async (t, { nft }) => {
  const totalSupply = await nft.view('nft_total_supply') as string;
  t.is(totalSupply, '4');
});

workspace.test('nft_tokens', async (t, { nft }) => {
  // No optional args should return all
  let tokens = await nft.view('nft_tokens') as Token[];
  t.is(tokens.length, 4);

  // Start at "1", with no limit arg
  tokens = await nft.view('nft_tokens', { from_index: '1' }) as Token[];
  t.is(tokens.length, 3);
  t.is(tokens[0].token_id, '1');
  t.is(tokens[1].token_id, '2');
  t.is(tokens[2].token_id, '3');

  // Start at "2", with limit 1
  tokens = await nft.view('nft_tokens', { from_index: '2', limit: 1 }) as Token[];
  t.is(tokens.length, 1);
  t.is(tokens[0].token_id, '2');

  // Don't specify from_index, but limit 2
  tokens = await nft.view('nft_tokens', { limit: 2 }) as Token[];
  t.is(tokens.length, 2);
  t.is(tokens[0].token_id, '0');
  t.is(tokens[1].token_id, '1');
});

workspace.test('nft_tokens_for_owner', async (t, { nft, alice, root }) => {
  // Requires `account_id`
  const error = await t.throwsAsync(
    nft.view('nft_tokens_for_owner')
  );
  t.regex(error.message, /missing field `account_id`/);

  // Get tokens from account with no NFTs
  let tokens =
    await nft.view('nft_tokens_for_owner', { account_id: alice }) as Token[];
  t.is(tokens.length, 0);

  // Get tokens with no optional args
  tokens =
    await nft.view('nft_tokens_for_owner', { account_id: root }) as Token[];
  t.is(tokens.length, 4);

  // With from_index and no limit
  // Start at "1", with no limit arg
  tokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: '2' }) as Token[];
  t.is(tokens.length, 2);
  t.is(tokens[0].token_id, '2');
  t.is(tokens[1].token_id, '3');

  // With from_index and limit 1
  tokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: '1', limit: 1 }) as Token[];
  t.is(tokens.length, 1);
  t.is(tokens[0].token_id, '1');

  // No from_index but limit 3
  tokens = await nft.view('nft_tokens_for_owner', { account_id: root, limit: 3 }) as Token[];
  t.is(tokens.length, 3);
  t.is(tokens[0].token_id, '0');
  t.is(tokens[1].token_id, '1');
  t.is(tokens[2].token_id, '2');
});
