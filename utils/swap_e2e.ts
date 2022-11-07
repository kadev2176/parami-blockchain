import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { system } from "@polkadot/types/interfaces/definitions";
import { mnemonicGenerate } from "@polkadot/util-crypto";
import { exit } from "process";
import Spinnies from "spinnies";
import { submit } from "./utils";

(async () => {
    const spinnies = new Spinnies();
  
    const provider = new WsProvider('ws://127.0.0.1:9944');
    const keyring = new Keyring({ type: 'sr25519' });
  
    const chain = await ApiPromise.create({ provider });
  
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

    spinnies.add('swap', {
      text: '          SWAP: pending...', 
    })
  
    // 1. New User
  
    spinnies.add('preparing', {
      text: 'Registering DID...',
    });
    await submit(
      chain,
      chain.tx.balances.transfer(m.address, 30_000n * 10n ** 18n),
      alice
    );
    await submit(chain, chain.tx.did.register(null), m);
    spinnies.remove('preparing');
    const didOf = await chain.query.did.didOf(m.address);
    const did = didOf.toString();
    spinnies.succeed('did', { text: ` User DID: ${did}` });
  
    // 2. Prepare Token
  
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
      text: 'Backing KOL...',
    });
    await submit(chain, chain.tx.nft.back(nft, 1_000n * 10n ** 18n), m);
  
    spinnies.update('preparing', {
      text: 'Minting...',
    });
    await submit(chain, chain.tx.nft.mint(nft, 'Test Token', 'XTT', 1000), k);
    spinnies.remove('preparing');
    spinnies.succeed('nft');
    spinnies.succeed('kol');
  
    // 3. buy tokens
    spinnies.add('preparing', {
        text: 'Buy tokens...',
    });
    await submit(chain, chain.tx.swap.buyTokens(nft, 1_000n * 10n ** 18n, 2n * 10n ** 18n, 40_000), m);

    console.log(`buy token success!`);

    spinnies.update('preparing', {
        text: 'Add liquiditiy...',
    });
    await submit(chain, chain.tx.swap.addLiquidity(nft, 1n * 10n ** 18n, 9n * 10n ** 17n, 1_500n * 10n ** 18n, 30_000), m);

    let resArray = await chain.query.swap.account.entries(m.address);
    let lp_token_ids = resArray.map(t => {
        let a = t[0].toHuman();
        console.log(`a is ${a}`);
        let b = a?.toString().split(',')[1];
        console.log(`b is ${b}`);
        return b;
    });

    spinnies.update('swap', {text: `lp_token_id: ${lp_token_ids[0]}`});

    console.log(`add liquidity success!`);

    spinnies.update('preparing', {
        text: 'remove liquiditiy...',
    });
    
    await submit(chain, chain.tx.swap.removeLiquidity(lp_token_ids[0], 9n * 10n ** 17n, 8n * 10n ** 17n, 40_000), m);

    console.log(`remove liquidity success!`);

    spinnies.remove('preparing');
    spinnies.succeed('swap');

    await chain.disconnect();
  })();