import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SubscriptionProgram } from "../target/types/subscription_program";
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import chai, { expect } from "chai";
import chaiAsPromised from "chai-as-promised";
chai.use(chaiAsPromised);
// Configure the client to use the local cluster.
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace
  .SubscriptionProgram as Program<SubscriptionProgram>;

const connection = anchor.getProvider().connection;

interface PlanConfig {
  term: "oneWeek" | "oneSecond";
}

const createPlan = async (config: Partial<PlanConfig> = {}) => {
  const owner = anchor.web3.Keypair.generate();
  const code = "test";
  const [plan_account] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from(anchor.utils.bytes.utf8.encode("plan")),
      owner.publicKey.toBuffer(),
      Buffer.from(anchor.utils.bytes.utf8.encode(code)),
    ],
    program.programId
  );
  const airdropTx = await connection.requestAirdrop(
    owner.publicKey,
    2000000000
  );
  const decimals = 9;
  await connection.confirmTransaction(airdropTx);
  const mint = await createMint(
    connection,
    owner,
    owner.publicKey,
    null,
    decimals
  );
  const ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    owner,
    mint,
    owner.publicKey,
    true
  );

  await program.methods
    .createPlan({
      code,
      price: new anchor.BN(10 * 10 ** decimals),
      term: { [config.term || "oneWeek"]: {} },
    })
    .accounts({
      payer: owner.publicKey,
      planAccount: plan_account,
      settlementTokenAccount: ownerTokenAccount.address,
    })
    .signers([owner])
    .rpc();

  return {
    owner,
    plan_account,
    mint,
    ownerTokenAccount,
  };
};

interface CreateSubscriptionData {
  owner: Keypair;
  mint: PublicKey;
  planAccount: PublicKey;
  ownerTokenAccount: PublicKey;
}

const createSubscription = async (data: CreateSubscriptionData) => {
  const { mint, owner, planAccount, ownerTokenAccount } = data;
  const payer = anchor.web3.Keypair.generate();
  const airdropTx = await connection.requestAirdrop(
    payer.publicKey,
    2000000000
  );
  await connection.confirmTransaction(airdropTx);
  const payerTokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    payer.publicKey,
    true
  );
  await mintTo(
    connection,
    payer,
    mint,
    payerTokenAccount.address,
    owner,
    100 * 10 ** 9
  );
  const [subscriptionAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from(anchor.utils.bytes.utf8.encode("subscription")),
      payer.publicKey.toBuffer(),
      planAccount.toBuffer(),
    ],
    program.programId
  );
  await program.methods
    .createSubscription({
      delegationAmount: new anchor.BN(100000 * 10 ** 9),
    })
    .accounts({
      payer: payer.publicKey,
      payerTokenAccount: payerTokenAccount.address,
      planAccount: planAccount,
      subscriptionAccount,
      settlementTokenAccount: ownerTokenAccount,
    })
    .signers([payer])
    .rpc();

  return {
    subscriptionAccount,
    payer,
    payerTokenAccount,
  };
};

describe("subscription-program", () => {
  it("Creates Plan", async () => {
    // Add your test here.
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan();
    const data = await program.account.plan.fetch(plan_account);
    console.log(data);
  });

  it("Creates a subscription", async () => {
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan();
    const { subscriptionAccount } = await createSubscription({
      owner,
      mint,
      planAccount: plan_account,
      ownerTokenAccount: ownerTokenAccount.address,
    });
    const data = await program.account.subscription.fetch(subscriptionAccount);
  });

  it("Fails to charge before appropriate time", async () => {
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan({
      term: "oneWeek",
    });
    const { subscriptionAccount, payerTokenAccount } = await createSubscription(
      {
        owner,
        mint,
        planAccount: plan_account,
        ownerTokenAccount: ownerTokenAccount.address,
      }
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    await expect(
      program.methods
        .chargeSubscription()
        .accounts({
          payer: random.publicKey,
          planAccount: plan_account,
          subscriptionAccount,
          settlementTokenAccount: ownerTokenAccount.address,
          subscriberTokenAccount: payerTokenAccount.address,
        })
        .signers([random])
        .rpc()
    ).to.eventually.rejected;
  });

  it("Charges subscription after one second", async () => {
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan({
      term: "oneSecond",
    });
    const { subscriptionAccount, payerTokenAccount } = await createSubscription(
      {
        owner,
        mint,
        planAccount: plan_account,
        ownerTokenAccount: ownerTokenAccount.address,
      }
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    await program.methods
      .chargeSubscription()
      .accounts({
        payer: random.publicKey,
        planAccount: plan_account,
        subscriptionAccount,
        settlementTokenAccount: ownerTokenAccount.address,
        subscriberTokenAccount: payerTokenAccount.address,
      })
      .signers([random])
      .rpc();
  });
});
