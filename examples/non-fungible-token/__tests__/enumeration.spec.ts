import path from 'path';
import { NEAR } from 'near-units';
import type { NearAccount } from 'near-runner-jest';
import type { Token } from './utils';
import { createRunner, TOKEN_ID } from './utils';

async function mintMore(root: NearAccount, nft: NearAccount) {
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
}

describe('1 token to start', () => {
  const runner = createRunner();

  runner.test('nft_supply_for_owner', async ({ root, alice, nft }) => {
    let n = await nft.view('nft_supply_for_owner', { account_id: alice }) as string;
    expect(n).toBe('0');

    n = await nft.view('nft_supply_for_owner', { account_id: root }) as string;
    expect(n).toBe('1');

    await mintMore(root, nft);

    n = await nft.view('nft_supply_for_owner', { account_id: root }) as string;
    expect(n).toBe('4');
  });
});

describe('4 tokens to start', () => {
  const runner = createRunner(async ({ root, nft }) => {
    await mintMore(root, nft);
  });

  runner.test('nft_total_supply', async ({ nft }) => {
    const totalSupply = await nft.view('nft_total_supply') as string;
    expect(totalSupply).toBe('4');
  });

  runner.test('nft_tokens', async ({ nft }) => {
    // No optional args should return all
    let tokens = await nft.view('nft_tokens') as Token[];
    expect(tokens.length).toBe(4);

    // Start at "1", with no limit arg
    tokens = await nft.view('nft_tokens', { from_index: '1' }) as Token[];
    expect(tokens.length).toBe(3);
    expect(tokens[0].token_id).toBe('1');
    expect(tokens[1].token_id).toBe('2');
    expect(tokens[2].token_id).toBe('3');

    // Start at "2", with limit 1
    tokens = await nft.view('nft_tokens', { from_index: '2', limit: 1 }) as Token[];
    expect(tokens.length).toBe(1);
    expect(tokens[0].token_id).toBe('2');

    // Don't specify from_index, but limit 2
    tokens = await nft.view('nft_tokens', { limit: 2 }) as Token[];
    expect(tokens.length).toBe(2);
    expect(tokens[0].token_id).toBe('0');
    expect(tokens[1].token_id).toBe('1');
  });

  runner.test('nft_tokens_for_owner', async ({ nft, alice, root }) => {
    // Requires `account_id`
    await expect(
      nft.view('nft_tokens_for_owner')
    ).rejects.toThrowError('missing field `account_id`');

    // Get tokens from account with no NFTs
    let tokens =
      await nft.view('nft_tokens_for_owner', { account_id: alice }) as Token[];
    expect(tokens.length).toBe(0);

    // Get tokens with no optional args
    tokens =
      await nft.view('nft_tokens_for_owner', { account_id: root }) as Token[];
    expect(tokens.length).toBe(4);

    // With from_index and no limit
    // Start at "1", with no limit arg
    tokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: '2' }) as Token[];
    expect(tokens.length).toBe(2);
    expect(tokens[0].token_id).toBe('2');
    expect(tokens[1].token_id).toBe('3');

    // With from_index and limit 1
    tokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: '1', limit: 1 }) as Token[];
    expect(tokens.length).toBe(1);
    expect(tokens[0].token_id).toBe('1');

    // No from_index but limit 3
    tokens = await nft.view('nft_tokens_for_owner', { account_id: root, limit: 3 }) as Token[];
    expect(tokens.length).toBe(3);
    expect(tokens[0].token_id).toBe('0');
    expect(tokens[1].token_id).toBe('1');
    expect(tokens[2].token_id).toBe('2');
  });
});