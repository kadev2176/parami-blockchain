import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import Spinnies from 'spinnies';

import { submit } from './utils.js';

(async () => {
  const spinnies = new Spinnies();

  const provider = new WsProvider('ws://localhost:9944');
  const keyring = new Keyring({ type: 'sr25519' });

  const chain = await ApiPromise.create({ provider });

  const alice = keyring.addFromUri('//Alice');

  const m = keyring.addFromUri(await mnemonicGenerate(12));
  const c = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('root', {
    text: ` Root Account: ${m.address}`,
    status: 'succeed',
  });

  spinnies.add('codo', {
    text: ` Codo Account: ${c.address}`,
    status: 'succeed',
  });

  spinnies.add('cash', {
    text: ' Cash Account: pending...',
  });

  spinnies.add('did', {
    text: '          DID: pending...',
  });

  spinnies.add('kol', {
    text: '          KOL: pending...',
  });

  spinnies.add('ad', {
    text: 'Advertisement: pending...',
  });

  // 1. New User

  spinnies.add('preparing', {
    text: 'Airdropping...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(c.address, 3_000_000_000_000_000_000_000n),
    alice
  );
  spinnies.remove('preparing');

  await submit(
    chain,
    chain.tx.magic.createStableAccount(
      m.address,
      2_000_000_000_000_000_000_000n
    ),
    c
  );
  const stableOf = await chain.query.magic.stableAccountOf(c.address);
  const s = stableOf.toHuman().stashAccount;
  spinnies.succeed('cash', { text: ` Cash Account: ${s}` });

  await submit(chain, chain.tx.magic.codo(chain.tx.did.register(null)), c);
  const didOf = await chain.query.did.didOf(s);
  const did = didOf.toString();
  spinnies.succeed('did', { text: `          DID: ${did}` });

  // 2. Prepare KOL

  spinnies.add('preparing', {
    text: 'Registering DID for Alice...',
  });
  await submit(chain, chain.tx.did.register(null), alice);
  const kolOf = await chain.query.did.didOf(alice.address);
  const kol = kolOf.toString();
  spinnies.update('kol', { text: `          KOL: ${kol}` });

  spinnies.update('preparing', {
    text: 'Backing KOL...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.nft.back(kol, 1_000_000_000_000_000_000_000n)),
    c
  );

  spinnies.update('preparing', {
    text: 'Minting...',
  });
  await submit(chain, chain.tx.nft.mint('Alice', 'XAA'), alice);
  spinnies.succeed('kol');

  // 3. Prepare Advertisement

  spinnies.update('preparing', {
    text: 'Depositing to become Advertiser...',
  });
  await submit(
    chain,
    chain.tx.advertiser.deposit(1_000_000_000_000_000_000_000n),
    alice
  );

  spinnies.update('preparing', {
    text: 'Creating Tags...',
  });
  await submit(chain, chain.tx.tag.create('Polkadot'), alice);

  spinnies.update('preparing', {
    text: 'Creating Advertisement...',
  });
  await submit(
    chain,
    chain.tx.ad.create(
      2_000_000_000_000_000_000n,
      ['Polkadot'],
      'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
      0,
      500000
    ),
    alice
  );
  const adsOf = await chain.query.ad.adsOf.entries();
  const ad = adsOf[0][1].toHuman()[0];
  spinnies.update('ad', { text: `Advertisement: ${ad}` });

  spinnies.update('preparing', {
    text: 'Bidding...',
  });
  await submit(
    chain,
    chain.tx.ad.bid(ad, kol, 1_000_000_000_000_000_000n),
    alice
  );
  spinnies.succeed('ad');

  spinnies.remove('preparing');

  // 4. Payout

  spinnies.add('pay', { text: 'Paying...' });
  const before = await chain.query.assets.account(0, s);
  const beforeBalance = before.balance.toHuman().replaceAll(',', '');
  await submit(
    chain,
    chain.tx.ad.pay(ad, kol, did, [['Polkadot', 5]], null),
    alice
  );
  const after = await chain.query.assets.account(0, s);
  const afterBalance = after.balance.toHuman().replaceAll(',', '');
  spinnies.succeed('pay', { text: `Paid ${afterBalance - beforeBalance}` });

  await chain.disconnect();
})();
