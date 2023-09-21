import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SubscriptionProgram } from "../target/types/subscription_program";
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import chai, { expect } from "chai";
import chaiAsPromised from "chai-as-promised";
import { readFileSync } from "fs";

chai.use(chaiAsPromised);
// Configure the client to use the local cluster.
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace
  .SubscriptionProgram as Program<SubscriptionProgram>;

const connection = anchor.getProvider().connection;

const deployer = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(readFileSync("./deployer.json", "utf-8")))
);

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
  const planTokenAccount = getAssociatedTokenAddressSync(
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
      planTokenAccount: planTokenAccount,
      mintAccount: mint,
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
      planTokenAccount,
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
        planTokenAccount,
      }
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    const deployerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      deployer,
      mint,
      deployer.publicKey
    );
    const ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      owner,
      mint,
      owner.publicKey,
      true
    );

    await expect(
      program.methods
        .chargeSubscription()
        .accounts({
          payer: random.publicKey,
          planAccount: plan_account,
          subscriptionAccount,
          planTokenAccount,
          subscriberTokenAccount: payerTokenAccount.address,
          deployerTokenAccount: deployerTokenAccount.address,
          ownerTokenAccount: ownerTokenAccount.address,
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
        planTokenAccount,
      }
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    const ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      owner,
      mint,
      owner.publicKey,
      true
    );
    const deployerAirdropTx = await connection.requestAirdrop(
      deployer.publicKey,
      2000000000
    );
    await connection.confirmTransaction(deployerAirdropTx);
    const deployTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      deployer,
      mint,
      deployer.publicKey
    );
    await program.methods
      .chargeSubscription()
      .accounts({
        payer: random.publicKey,
        planAccount: plan_account,
        subscriptionAccount,
        planTokenAccount,
        subscriberTokenAccount: payerTokenAccount.address,
        ownerTokenAccount: ownerTokenAccount.address,
        deployerTokenAccount: deployTokenAccount.address,
      })
      .signers([random])
      .rpc();

    const escrowBalance = await connection.getTokenAccountBalance(
      planTokenAccount
    );
    expect(escrowBalance.value.uiAmount).to.eq(10);
    const ownerBalance = await connection.getTokenAccountBalance(
      ownerTokenAccount.address
    );
    expect(ownerBalance.value.uiAmount).to.eq(9.7);
    const deployerBalance = await connection.getTokenAccountBalance(
      deployTokenAccount.address
    );
    expect(deployerBalance.value.uiAmount).to.eq(0.3);
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
        planTokenAccount: planTokenAccount,
        amount: 15,
      }
    );
    const random = anchor.web3.Keypair.generate();
    const airdropTx = await connection.requestAirdrop(
      random.publicKey,
      2000000000
    );
    await connection.confirmTransaction(airdropTx);
    const ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      owner,
      mint,
      owner.publicKey,
      true
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const deployerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      deployer,
      mint,
      deployer.publicKey
    );
    await program.methods
      .chargeSubscription()
      .accounts({
        payer: random.publicKey,
        planAccount: plan_account,
        subscriptionAccount,
        planTokenAccount: planTokenAccount,
        subscriberTokenAccount: payerTokenAccount.address,
        ownerTokenAccount: ownerTokenAccount.address,
        deployerTokenAccount: deployerTokenAccount.address,
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
        planTokenAccount: planTokenAccount,
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
        planTokenAccount: planTokenAccount,
        amount: 10,
      });
    const planOwnerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      owner,
      mint,
      owner.publicKey,
      true
    );
    const deployerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      deployer,
      mint,
      deployer.publicKey
    );
    await new Promise((resolve) => setTimeout(resolve, 5000));
    const ret = await program.methods
      .closeSubscription()
      .accounts({
        planAccount: plan_account,
        payer: payer.publicKey,
        payerTokenAccount: payerTokenAccount.address,
        subscriptionAccount: subscriptionAccount,
        planTokenAccount,
        subscriberTokenAccount: payerTokenAccount.address,
        planOwnerTokenAccount: planOwnerTokenAccount.address,
        deployerTokenAccount: deployerTokenAccount.address,
      })
      .signers([payer])
      .rpc();
    const payerBalance = await connection.getTokenAccountBalance(
      payerTokenAccount.address
    );
    const ownerBalance = await connection.getTokenAccountBalance(
      planOwnerTokenAccount.address
    );
    const escrowBalance = await connection.getTokenAccountBalance(
      planTokenAccount
    );
    const deployerBalance = await connection.getTokenAccountBalance(
      deployerTokenAccount.address
    );
    expect(escrowBalance.value.uiAmount).to.eq(0);
    expect(ownerBalance.value.uiAmount + payerBalance.value.uiAmount).to.eq(
      10 - deployerBalance.value.uiAmount
    );
    expect(payerBalance.value.uiAmount).to.be.gt(ownerBalance.value.uiAmount);
    expect(deployerBalance.value.uiAmount).to.gt(0);
  });
});
