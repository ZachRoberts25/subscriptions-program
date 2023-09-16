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
  term: "oneWeek" | "oneSecond" | "thirtySeconds";
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
  const planTokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    owner,
    mint,
    plan_account,
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
      planTokenAccount: planTokenAccount.address,
    })
    .signers([owner])
    .rpc();

  return {
    owner,
    plan_account,
    mint,
    planTokenAccount,
  };
};

interface CreateSubscriptionData {
  owner: Keypair;
  mint: PublicKey;
  planAccount: PublicKey;
  planTokenAccount: PublicKey;
  amount?: number;
}

const createSubscription = async (data: CreateSubscriptionData) => {
  const { mint, owner, planAccount, planTokenAccount } = data;
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
    (data.amount || 100) * 10 ** 9
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
      planTokenAccount: planTokenAccount,
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
    const { plan_account, mint, owner, planTokenAccount } = await createPlan();
    const data = await program.account.plan.fetch(plan_account);
    console.log(data);
  });

  it("Creates a subscription", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan();
    const { subscriptionAccount } = await createSubscription({
      owner,
      mint,
      planAccount: plan_account,
      planTokenAccount: planTokenAccount.address,
    });
    const data = await program.account.subscription.fetch(subscriptionAccount);
  });

  it("Fails to charge before appropriate time", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan({
      term: "oneWeek",
    });
    const { subscriptionAccount, payerTokenAccount } = await createSubscription(
      {
        owner,
        mint,
        planAccount: plan_account,
        planTokenAccount: planTokenAccount.address,
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
          planTokenAccount: planTokenAccount.address,
          subscriberTokenAccount: payerTokenAccount.address,
        })
        .signers([random])
        .rpc()
    ).to.eventually.rejected;
  });

  it("Charges subscription after one second", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan({
      term: "oneSecond",
    });
    const { subscriptionAccount, payerTokenAccount } = await createSubscription(
      {
        owner,
        mint,
        planAccount: plan_account,
        planTokenAccount: planTokenAccount.address,
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
        planTokenAccount: planTokenAccount.address,
        subscriberTokenAccount: payerTokenAccount.address,
      })
      .signers([random])
      .rpc();
  });

  it("Handles Past Due", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan({
      term: "oneSecond",
    });
    const { subscriptionAccount, payerTokenAccount } = await createSubscription(
      {
        owner,
        mint,
        planAccount: plan_account,
        planTokenAccount: planTokenAccount.address,
        amount: 15,
      }
    );
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    await program.methods
      .chargeSubscription()
      .accounts({
        payer: random.publicKey,
        planAccount: plan_account,
        subscriptionAccount,
        planTokenAccount: planTokenAccount.address,
        subscriberTokenAccount: payerTokenAccount.address,
      })
      .signers([random])
      .rpc();

    const data = await program.account.subscription.fetch(subscriptionAccount);
    expect(!!data.state.pastDue).to.eq(true);
  });

  it("Cancels & uncancels a subscription", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan();

    const { subscriptionAccount, payerTokenAccount, payer } =
      await createSubscription({
        owner,
        mint,
        planAccount: plan_account,
        planTokenAccount: planTokenAccount.address,
      });

    await program.methods
      .cancelSubscription()
      .accounts({
        planAccount: plan_account,
        payer: payer.publicKey,
        subscriptionAccount: subscriptionAccount,
      })
      .signers([payer])
      .rpc();
    const data = await program.account.subscription.fetch(subscriptionAccount);
    expect(!!data.state.pendingCancellation).to.eq(true);
    await program.methods
      .uncancelSubscription()
      .accounts({
        planAccount: plan_account,
        payer: payer.publicKey,
        subscriptionAccount: subscriptionAccount,
      })
      .signers([payer])
      .rpc();
    const data2 = await program.account.subscription.fetch(subscriptionAccount);
    expect(!!data2.state.active).to.eq(true);
  });

  it("Closes subscription and provides refund", async () => {
    const { plan_account, mint, owner, planTokenAccount } = await createPlan({
      term: "thirtySeconds",
    });

    const { subscriptionAccount, payerTokenAccount, payer } =
      await createSubscription({
        owner,
        mint,
        planAccount: plan_account,
        planTokenAccount: planTokenAccount.address,
        amount: 100,
      });
    await new Promise((resolve) => setTimeout(resolve, 5000));
    const ret = await program.methods
      .closeSubscription()
      .accounts({
        planAccount: plan_account,
        payer: payer.publicKey,
        payerTokenAccount: payerTokenAccount.address,
        subscriptionAccount: subscriptionAccount,
        planTokenAccount: planTokenAccount.address,
        subscriberTokenAccount: payerTokenAccount.address,
      })
      .signers([payer])
      .rpc();
    console.log(ret);
    const balance = await connection.getTokenAccountBalance(
      payerTokenAccount.address
    );
    // they got a refund for 5ish seconds of a 30 second sub, it'll be somewhere between 90 and 100;
    expect(balance.value.uiAmount).to.be.gt(100 - 10);
    expect(balance.value.uiAmount).to.be.lt(100);
  });
});
