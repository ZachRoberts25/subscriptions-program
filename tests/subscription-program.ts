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
  const plan_account = anchor.web3.Keypair.generate();
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
      code: "Test Plan",
      price: new anchor.BN(10 * 10 ** decimals),
      term: { [config.term || "oneWeek"]: {} },
    })
    .accounts({
      payer: owner.publicKey,
      planAccount: plan_account.publicKey,
      settlementTokenAccount: ownerTokenAccount.address,
    })
    .signers([owner, plan_account])
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
  const subscription_account = anchor.web3.Keypair.generate();
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
  const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
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
      subscriptionAccount: subscription_account.publicKey,
      pdaAccount: pda,
      settlementTokenAccount: ownerTokenAccount,
    })
    .signers([payer, subscription_account])
    .rpc();

  return {
    subscription_account,
    payer,
    payerTokenAccount,
    pda,
  };
};

describe("subscription-program", () => {
  it("Creates Plan", async () => {
    // Add your test here.
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan();
    const data = await program.account.plan.fetch(plan_account.publicKey);
  });

  it("Creates a subscription", async () => {
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan();
    const { subscription_account } = await createSubscription({
      owner,
      mint,
      planAccount: plan_account.publicKey,
      ownerTokenAccount: ownerTokenAccount.address,
    });
    const data = await program.account.subscription.fetch(
      subscription_account.publicKey
    );
  });

  it("Fails to charge before appropriate time", async () => {
    const { plan_account, mint, owner, ownerTokenAccount } = await createPlan({
      term: "oneWeek",
    });
    const { subscription_account, pda, payerTokenAccount } =
      await createSubscription({
        owner,
        mint,
        planAccount: plan_account.publicKey,
        ownerTokenAccount: ownerTokenAccount.address,
      });
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
          pdaAccount: pda,
          planAccount: plan_account.publicKey,
          subscriptionAccount: subscription_account.publicKey,
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
    const { subscription_account, pda, payerTokenAccount } =
      await createSubscription({
        owner,
        mint,
        planAccount: plan_account.publicKey,
        ownerTokenAccount: ownerTokenAccount.address,
      });
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
        pdaAccount: pda,
        planAccount: plan_account.publicKey,
        subscriptionAccount: subscription_account.publicKey,
        settlementTokenAccount: ownerTokenAccount.address,
        subscriberTokenAccount: payerTokenAccount.address,
      })
      .signers([random])
      .rpc();
  });
});
