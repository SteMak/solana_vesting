# Solana Vesting SC

This example vesting project supports multisig funders and receivers.

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

// ["DISTRIBUTE", seed_key]
// PDA for unlocked funds (token account),
// authority is set to the vesting beneficiary
```

The distribute account is implemented as a PDA to prevent unauthorized transfers of funds to the personal beneficiary account and to enable multisig support.

For the same reason, the vault account does not need to be funded upon vesting creation; the multisignature wallet needs to top up the vault after the vesting is created.

The program supports both underfunded and overfunded vault cases without failure. If the vault is not fully funded, unlocks occur depending on the availability of funds. If excess tokens are deposited into the vault, they are released as soon as the vesting period ends.
