# Solana Vesting SC

Example Vesting SC on Solana

Vesting funds are distributed linearly since the start and to the end with a specified cliff

Supports

- Multisig funders
- Multisig receivers
- Partial funding

## Disclaimer

The Solana native framework is quite low-level. While this aids in understanding the main principles of the platform, it also presents various challenges for development. It should be noted that some issues may not be identifiable without integration testing, meaning that code which appears straightforward and compiles correctly may still partially fail to work.

It is recommended to cover your code with integration tests and perform a beta test on the testnet before going live on the mainnet.

One more note about Anchor: While it abstracts many low-level operations and provides a unified view for any smart contract, it introduces a lot of complexity. Additionally, it does not offer a testing framework that supports coverage measurement.

## SC Overview

Each vesting is isolated for security reasons. All PDAs using the same `seed_key` correspond to the same vesting.

```rust
// ["VESTING", seed_key]
// PDA for vesting data,
// reasonable to filter by `beneficiary`, `mint`, `creator`
pub struct Vesting {
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub seed_key: Pubkey,
    pub creator: Pubkey,

    pub amount: u64,
    pub claimed: u64,

    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
}

// ["VAULT", seed_key]
// PDA for locked funds (token account),
// each vesting amount is isolated and supports partial funding
spl_token::state::Account

// ["DISTRIBUTE", seed_key]
// PDA for unlocked funds (token account),
// authority is set to the vesting beneficiary
spl_token::state::Account
```

The distribute account is implemented as a PDA to prevent unauthorized transfers of funds to the personal beneficiary account and to enable beneficiary multisig support. The account belongs to the beneficiary and allows for transferring unlocked funds using the SPL token program.

For the same reason, the vault account is not funded during vesting creation. The multisignature wallet needs to top up the vault after the vesting is created.

The program supports both underfunded and overfunded vault cases without failure. If the vault is not fully funded, unlocks occur depending on the availability of funds. If excess tokens are deposited into the vault, they are released as soon as the vesting period ends.

## Notes

The multisig wallet is an SPL token program PDA, which cannot be a signer for vesting create or claim transactions. The funds transfers require a varying number of signers depending on the multisig configuration. To mitigate issues of processing different amounts of signers in this contract, the easiest way is to provide a token account where the multisig needs to deposit or from which it is able to withdraw.

Partial funding support is a consequence of multisig support. Developing stages with limited time periods is excessive, as it is easy to support a partially funded vault account.

Overfunding requires manual funding. Excessive tokens will definitely be sent to the vault as funding happens manually. In the implementation, the funds are unlocked to the recipient at the vesting end in the form of an additional bonus.

The PDA module is what I'm proud of. It wraps all data operations, seed checks, and useful features like PDA creation and token transfers.

The instruction module contains a lot of lifetime-related features. It wasn't the best idea, but it nevertheless well-structures the accounts needed by the instructions.

Integration tests can be written in a way that does not require `cargo-test-sbf` and thus are included in coverage calculations.

```sh
// Build
cargo build

// Test
cargo test

// Measure coverage
cargo +nightly llvm-cov
```
