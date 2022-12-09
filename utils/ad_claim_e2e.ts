import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import Spinnies from 'spinnies';
import { u8aToHex, hexToU8a, stringToU8a } from '@polkadot/util';
import { keccak256AsU8a, keccakAsHex } from '@polkadot/util-crypto';
import * as $ from "parity-scale-codec";
import { submit, submitUntilFinalized } from './utils';
import assert from "node:assert"

(async () => {
  const spinnies = new Spinnies();

  const provider = new WsProvider('ws://127.0.0.1:9944');
  const keyring = new Keyring({ type: 'sr25519' });

    const chain = await ApiPromise.create({ provider,
        runtime: {
            ClockInRuntimeApi: [{
                methods: {
                    get_clock_in_info: {
                        description: "(enabled, claimable, token)",
                        params: [
                            {
                                "name": "nft_id",
                                "type": "u32"
                            },
                            {
                                "name": "did",
                                "type": "H160"
                            }
                        ],
                        type: "(u8, bool, bool, u128)"
                    }
                },
                version: 1
            }]
          },
    });

  const alice = keyring.addFromUri('//Alice');

  const m = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('root', {
    text: ` User Account: ${m.address}`,
    status: 'succeed',
  });

  spinnies.add('did', {
    text: ' User DID: pending...',
  });

  spinnies.add('kol', {
    text: '          KOL: pending...',
  });

  spinnies.add('nft', {
    text: '          NFT: pending...',
  });

  spinnies.add('advertiser', {
    text: '   Advertiser: pending...',
  });

  spinnies.add('ad', {
    text: 'Advertisement: pending...',
  });

  // 1. New User

  spinnies.add('preparing', {
    text: 'Registering DID...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(m.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), m);
  spinnies.remove('preparing');
  const didOf = await chain.query.did.didOf(m.address);
  const did = didOf.toString();
  spinnies.succeed('did', { text: ` User DID: ${did}` });

  // 2. Prepare KOL

  const k = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating KOL...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(k.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), k);
  const kolOf = await chain.query.did.didOf(k.address);
  const kol = kolOf.toString();
  spinnies.update('kol', { text: `          KOL: ${kol}` });

  spinnies.update('preparing', {
    text: 'Creating NFT...',
  });
  await submit(chain, chain.tx.nft.kick(), k);
  const nftOf = await chain.query.nft.preferred(kol);
  const nft = nftOf.toString();
  spinnies.update('nft', { text: `          NFT: ${nft}` });

  spinnies.update('preparing', {
    text: 'Minting...',
  });
    await submit(chain, chain.tx.nft.mintNftPower(nft, 'Test Token', 'XTT', 2000000n * (10n ** 18n)), k);
    await submit(chain, chain.tx.swap.create(nft), k);
    await submit(chain, chain.tx.swap.addLiquidity(nft, 1000n * (10n ** 18n), 1000, 1000000n * (10n **18n), 10000), k);
  spinnies.remove('preparing');
  spinnies.succeed('nft');
  spinnies.succeed('kol');

    let clockInInfo = (await chain.call.clockInRuntimeApi.getClockInInfo(nft, did)).toJSON() as any[];
    assert.ok(clockInInfo[1] == 0 && clockInInfo[2] == 0 && BigInt(clockInInfo[3]) == 0n);
    assert.ok(clockInInfo[1] == 0);
    assert.ok(clockInInfo[2] == 0);

    await submitUntilFinalized(chain, chain.tx.clockIn.enableClockIn(nft, [10, 20, 30, 40, 50], [1_000, 10_000, 100_000, 1_000_000, 10_000_000], 5, 100, 1000), k);
    clockInInfo = (await chain.call.clockInRuntimeApi.getClockInInfo(nft, did)).toJSON() as any[];
    assert.ok(clockInInfo[1] > 0);
    assert.ok(clockInInfo[2] > 0);
    assert.ok(BigInt(clockInInfo[3]) == 0n);

    await submitUntilFinalized(chain, chain.tx.clockIn.clockIn(nft), m);
    clockInInfo = (await chain.call.clockInRuntimeApi.getClockInInfo(nft, did)).toJSON() as any[];
    assert.ok(clockInInfo[1] > 0);
    assert.ok(clockInInfo[2] == 0);
    assert.ok(BigInt(clockInInfo[3]) == 0n);

  // 3. Prepare Advertiser

  const a = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating Advertiser...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(a.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), a);
  const aderOf = await chain.query.did.didOf(a.address);
  const ader = aderOf.toString();
  spinnies.update('advertiser', { text: `   Advertiser: ${ader}` });

  spinnies.update('preparing', {
    text: 'Depositing to become Advertiser...',
  });
  await submit(chain, chain.tx.advertiser.deposit(1_000n * 10n ** 18n), a);
  spinnies.remove('preparing');
  spinnies.succeed('advertiser');

  // 4. Prepare Advertisement

  const tag = new Date().toISOString();

  spinnies.add('preparing', {
    text: 'Creating Tags...',
  });
  await submit(chain, chain.tx.tag.create(tag), a);

  spinnies.update('preparing', {
    text: 'Creating Advertisement...',
  });
  await submit(
    chain,
    chain.tx.ad.create(
      [tag],
      'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
      10,
      500000,
      1n * 10n ** 18n,
      1n * 10n ** 18n,
      50n * 10n ** 18n,
      null
    ),
    a
  );
  const adsOf = await chain.query.ad.adsOf(ader);
  const ad = (adsOf.toHuman() as any)[0];
  spinnies.update('ad', { text: `Advertisement: ${ad}` });

  spinnies.update('preparing', {
    text: `Bidding by ${a.address}...`,
  });

  await submit(chain, chain.tx.swap.sellCurrency(nft, 40n * 10n ** 18n, 2_000n * 10n ** 18n, 50000), a);

  await submit(chain, chain.tx.ad.bidWithFraction(ad, nft, 1_000n * 10n ** 18n, null, null), a);
  spinnies.succeed('ad');

  spinnies.remove('preparing');

  // 4. generate advertiser's signature
  spinnies.add('generate Signature', { text: `Generating...`});
  const scores = [{tag: tag, score: 5}];
  const adIdU8a = hexToU8a(ad);
  const nftIdU8a = $.u32.encode(parseInt(nft));
  const didU8a = hexToU8a(did);

  // const scoresU8a =  new Uint8Array([...$.str.encode(tag), ...$.i8.encode(5)])
  const scoresU8a = scores.reduce((pre, current) => {
    return new Uint8Array([...pre, ...stringToU8a(current.tag), ...$.i8.encode(current.score)])
  }, new Uint8Array());

  let messageU8a = new Uint8Array([...adIdU8a, ...nftIdU8a, ...didU8a, ...scoresU8a]);

  console.log('message bytes', messageU8a);
  console.log('message hex', u8aToHex(messageU8a));

  const messageU8aHash = keccak256AsU8a(messageU8a);

  console.log('message hash u8a', messageU8aHash);
  console.log('message hash hex', u8aToHex(messageU8aHash));

  const signature = a.sign(messageU8aHash);

  spinnies.succeed(`generate Signature`);
  // 5. Payout

  spinnies.add('claim', { text: `Claiming...` });
  const before = await chain.query.assets.account(nft, m.address);
  const beforeBalance = !!before && !!(before as any).toHuman() ? (before as any).toHuman().balance : '0';

  await submit(chain, chain.tx.ad.claim(ad, nft, did, [[tag, 5]], null, { Sr25519:  signature }, a.address), a);

  const after = await chain.query.assets.account(nft, m.address);
  const { balance = '' } = (after as any).toHuman();
  const afterBalance = balance;
  spinnies.succeed('claim', { text: `Paid: before is ${beforeBalance}, after is ${afterBalance}` });

  // spinnies.add('pay', { text: `Paying to ${m.address}...` });
  // const before = await chain.query.assets.account(nft, m.address);
  // const beforeBalance = !!before && !!(before as any).toHuman() ? (before as any).toHuman().balance : '0';
  // await submit(chain, chain.tx.ad.pay(ad, nft, did, [[tag, 5]], null), a);
  // const after = await chain.query.assets.account(nft, m.address);
  // const { balance = '' } = (after as any).toHuman();
  // const afterBalance = balance;
  // spinnies.succeed('pay', { text: `Paid: before is ${beforeBalance}, after is ${afterBalance}` });

  await chain.disconnect();
})();
