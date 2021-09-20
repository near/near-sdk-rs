import path from 'path';
import { Runner, ReturnedAccounts } from 'near-runner-jest';
import { Gas } from 'near-units';

const TOKEN_ID = '0';

interface Token {
  token_id: string;
  owner_id: string;
  metadata?: TokenMetadata,
  approved_account_ids?: Record<string, number>;
}

interface TokenMetadata {
  title?: string;
  description?: string;
  media?: string;
  media_hash?: string;
  copies?: number;
  issued_at?: string;
  expires_at?: string;
  starts_at?: string;
  updated_at?: string;
  extra?: string;
  reference?: string;
  reference_hash?: string;
}

function createRunner(andThen: ((ReturnedAccounts) => Promise<ReturnedAccounts>) = async () => ({})) {
  return Runner.create(async ({ root }) => {
    const alice = await root.createAccount('alice');
    const nft = await root.createAndDeploy(
      'nft',
      path.join(__dirname, '..', 'res', 'non_fungible_token.wasm'),
      {
        method: 'new_default_meta',
        args: { owner_id: root }
      }
    );

    await root.call(
      nft,
      'nft_mint',
      {
        token_id: TOKEN_ID,
        token_owner_id: root,
        token_metadata: {
          title: 'Olympus Mons',
          description: 'The tallest mountain in the charted solar system',
          copies: 1,
        }
      },
      {
        attachedDeposit: '7000000000000000000000'
      }
    );

    return {
      ...await andThen({ alice, root, nft }),
      alice,
      nft
    }
  });
}

describe('nft_transfer', () => {
  const runner = createRunner();

  runner.test('success', async ({ alice, nft, root }) => {
    await root.call(
      nft,
      'nft_transfer',
      {
        receiver_id: alice,
        token_id: TOKEN_ID,
        memo: 'simple transfer',
      },
      {
        attachedDeposit: '1'
      }
    );

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(alice.accountId);
  });

  runner.test("failure; cannot send some else's token", async ({ alice, nft, root }) => {
    await expect(
      alice.call_raw( // alice tries to send it to herself
        nft,
        'nft_transfer',
        {
          receiver_id: alice,
          token_id: TOKEN_ID,
          memo: 'simple transfer',
        },
        {
          attachedDeposit: '1'
        }
      )
    ).rejects.toThrow('Unauthorized');

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });
});

describe('nft_transfer_call', () => {
  const runner = createRunner(async ({ root, nft }) => ({
    tokenReceiver: await root.createAndDeploy(
      'token-receiver',
      path.join(__dirname, '..', 'res', 'token_receiver.wasm'),
      {
        method: 'new',
        args: { non_fungible_token_account_id: nft }
      }
    )
  }));

  runner.test('refund (no cross-contract call from receiver)', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(
      nft,
      'nft_transfer_call',
      {
        receiver_id: tokenReceiver,
        token_id: TOKEN_ID,
        memo: 'transfer and call',
        msg: 'return-it-now',
      },
      {
        attachedDeposit: '1',
        gas: Gas.parse('31 Tgas'),
      }
    );

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });

  runner.test('refund after receiver makes cross-contract call', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(
      nft,
      'nft_transfer_call',
      {
        receiver_id: tokenReceiver,
        token_id: TOKEN_ID,
        memo: 'transfer and call',
        msg: 'return-it-later',
      },
      {
        attachedDeposit: '1',
        gas: Gas.parse('41 Tgas'),
      }
    );

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });

  runner.test('success (no cross-contract call from receiver)', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(
      nft,
      'nft_transfer_call',
      {
        receiver_id: tokenReceiver,
        token_id: TOKEN_ID,
        memo: 'transfer and call',
        msg: 'keep-it-now',
      },
      {
        attachedDeposit: '1',
        gas: Gas.parse('31 Tgas'),
      }
    );

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(tokenReceiver.accountId);
  });

  runner.test('success after receiver makes cross-contract call', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(
      nft,
      'nft_transfer_call',
      {
        receiver_id: tokenReceiver,
        token_id: TOKEN_ID,
        memo: 'transfer and call',
        msg: 'keep-it-later',
      },
      {
        attachedDeposit: '1',
        gas: Gas.parse('41 Tgas'),
      }
    );

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(tokenReceiver.accountId);
  });

  runner.test('refund if receiver panics', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(
      nft,
      'nft_transfer_call',
      {
        receiver_id: tokenReceiver,
        token_id: TOKEN_ID,
        memo: 'transfer and call',
        msg: 'keep-it-later',
      },
      {
        attachedDeposit: '1',
        gas: Gas.parse('40 Tgas'), // not enough gas!
      }
    );

    expect(tx.promiseErrors.length).toBe(1);
    expect(tx.promiseErrorMessages[0]).toMatch('FunctionCallZeroAttachedGas');

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });
});