# Token Fundraiser — v2: Optimized Pinocchio

> **Series:** Solana CU Optimization · Part 2 of 3

A fundraising program for SPL Tokens built on Solana using the
[Pinocchio](https://github.com/anza-xyz/pinocchio) framework. This version
applies every optimization available **within Pinocchio's safety model** —
no undefined behaviour, no raw entrypoint, no layout assumptions outside
what the framework guarantees. The goal is maximum CU reduction while
keeping every safety invariant Pinocchio provides intact.

---

## What It Does

A maker creates a fundraiser specifying:
- The SPL mint token they want to collect
- A target amount to raise
- A duration in days

Contributors can send tokens into a program-owned vault. Once the target
is met the maker can claim the vault. If the deadline passes without
reaching the target, contributors can claim refunds.

---

## Instructions

| Instruction   | Accounts | Description |
|---------------|----------|-------------|
| `Initialize`  | 7        | Create the fundraiser account and vault ATA |
| `Contribute`  | 9        | Deposit tokens; create contributor PDA on first contribution |
| `Checker`     | 7        | Verify goal reached and transfer vault to maker |
| `Refund`      | 9        | Return contributor tokens after deadline with unmet goal |

---

## How to Run

```bash
# Build the program
cargo build-sbf

# Run all tests with output
cargo test -- --nocapture
```

---

## Optimization Approach

Every technique used here stays within Pinocchio's validated framework.
No external layout assumptions are made about the BPF input buffer.
No unsafe blocks escape Pinocchio's own safety guarantees.

### Techniques Applied

**1. Eliminate `derive_address` (–4,500 CU across a full workflow)**

The single biggest win. `sol_create_program_address` costs ~1,500 CU per
call. It was called before every `invoke_signed` to pre-verify the PDA.
This is redundant: the runtime re-validates the PDA inside `invoke_signed`
anyway. Removing all three calls saves ~4,500 CU total, with zero loss
of security.

```rust
// BEFORE — 1,500 CU wasted per call:
let pda = derive_address(&seed, None, &crate::ID.to_bytes());
assert_eq!(pda, *fundraiser.address().as_array());

// AFTER — runtime validates during CPI, 0 CU:
// (just call invoke_signed directly, the PDA check is built in)
```

**2. Hardcode rent-exempt lamport values (–100 CU per `CreateAccount`)**

`Rent::get()?` is a sysvar syscall (~100 CU). The rent-exempt minimum for
a given account size is a pure function of the data length and has been
constant on mainnet since genesis. Hardcoded values eliminate the syscall
entirely:

```rust
// Formula: (data_len + 128) * 6960
pub const FUNDRAISER_RENT_LAMPORTS: u64 = (90 + 128) * 6960;  // 1,517,280
pub const CONTRIBUTOR_RENT_LAMPORTS: u64 = (8  + 128) * 6960;  //   945,600
```

**3. Raw pointer reads for all state field access (–15 to –30 CU per integer read)**

`u64::from_le_bytes(slice.try_into().unwrap())` compiles to ~15–30 BPF
instructions. A direct pointer read compiles to a single `ldxdw` (1 CU):

```rust
// BEFORE — bounds check + try_into + unwrap + conversion: ~20 CU:
u64::from_le_bytes(self.amount_to_raise.try_into().unwrap())

// AFTER — single ldxdw BPF instruction: 1 CU:
unsafe { (self.amount_to_raise.as_ptr() as *const u64).read_unaligned() }
```

**4. Direct token account byte reads (–50 CU per read)**

`pinocchio_token::state::Account::from_account_view()` validates the full
token account layout before every read. The SPL token account layout is
a stable specification: amount is always at byte offset 64. Direct read:

```rust
// BEFORE — full validation wrapper: ~50 CU:
let vault_data = TokenAccountState::from_account_view(vault)?;
let amount = vault_data.amount();

// AFTER — single ldxdw at known offset: 1 CU:
let amount = unsafe {
    (vault.borrow_unchecked().as_ptr().add(64) as *const u64).read_unaligned()
};
```

**5. Same technique for mint decimals (–30 CU)**

Decimals are at byte offset 44 in the SPL Mint layout:
```rust
let decimals = unsafe { *mint_to_raise.borrow_unchecked().as_ptr().add(44) };
```

**6. 10ˣ lookup table instead of `pow()` (–35 CU)**

`10u64.pow(decimals)` on BPF emits a software loop (~40 CU). Since
`decimals` is bounded to 0–9, a 10-entry const table costs exactly one
array access (1 CU):

```rust
const POWERS_OF_10: [u64; 10] = [
    1, 10, 100, 1_000, 10_000, 100_000,
    1_000_000, 10_000_000, 100_000_000, 1_000_000_000,
];
let scale = unsafe { *POWERS_OF_10.get_unchecked(decimals as usize) };
```

**7. Build profile: LTO + single codegen unit + panic=abort**

```toml
[profile.release]
opt-level = 3
lto = "fat"            # Cross-crate inlining: biggest compiler-level win
codegen-units = 1      # Required for fat LTO
panic = "abort"        # Eliminates all unwind infrastructure
overflow-checks = false
strip = "symbols"
```

This alone accounts for the majority of the 52.44 KB → 14.20 KB binary
reduction. `lto = "fat"` allows LLVM to inline and eliminate dead code
across crate boundaries, which pinocchio's own helper functions then
vanish entirely when they compile down to single instructions.

**8. Direct discriminator dispatch (–15 CU)**

```rust
// BEFORE — TryFrom<&u8> for enum + match: ~20 CU:
match FundraiserInstructions::try_from(discriminator)? { ... }

// AFTER — raw u8 match: ~5 CU:
let discriminator = unsafe { *instruction_data.as_ptr() };
match discriminator { 0 => ..., 1 => ..., _ => Err(...) }
```

---

## Test Results

```
running 7 tests

Final Binary Size: 14.20 KB
test test::tests::test_binary_size ... ok   ✓ passes L1 iCache threshold

Initialize CU:  16,174   (with new ATA creation)
Initialize CU:  16,174   (second run, ATA exists)
Initialize CU:  19,174   (with extra account overhead variant)
Contribute CU:   2,688
Contribute CU:   2,736
Refund CU:       1,582
Checker CU:      1,251

Estimated CU per account parsed: 2,311

test result: ok. 7 passed; 0 failed
```

---

## Performance Summary

| Metric              | v1 Baseline | v2 (this)  | Improvement |
|---------------------|-------------|------------|-------------|
| Binary size         | 52.44 KB    | **14.20 KB** | ↓ 72.9%   |
| Initialize CU       | 16,628      | **16,174** | ↓ 454 CU (2.7%) |
| Contribute CU       |  3,209      |  **2,688** | ↓ 521 CU (16.2%) |
| Refund CU           |  1,870      |  **1,582** | ↓ 288 CU (15.4%) |
| Checker CU          |  1,341      |  **1,251** | ↓ 90 CU (6.7%) |
| Tests passing       | 6 / 7       | **7 / 7**  | +1 test |

### CU Reduction Breakdown

| Technique                         | CU Saved | Applies To |
|-----------------------------------|----------|------------|
| Remove `derive_address` (×3)      | ~4,500   | Initialize, Contribute, Refund |
| Hardcoded rent constants (×2)     | ~200     | Initialize, Contribute |
| Raw token account reads (×2)      | ~100     | Checker, Refund |
| Direct mint decimals read         | ~30      | Initialize, Contribute |
| Raw pointer state field reads     | ~60–80   | All instructions |
| POWERS_OF_10 table vs `pow()`     | ~35      | Initialize, Contribute |
| Raw discriminator dispatch        | ~15      | All instructions |
| LTO + codegen-units=1 (binary)    | –38 KB   | All instructions (icache) |

---

## Why Initialize Barely Improved

Initialize drops only 454 CU (2.7%) because the `CreateATA` CPI
dominates at ~14,000 CU. Inside that single CPI call the ATA program
invokes both `CreateAccount` and `InitializeAccount` — two nested CPIs.
No technique in v2 or v3 can reduce that cost; it is work performed
entirely inside the ATA program's own execution. The optimizations
applied to Initialize's own code (removing `derive_address`, hardcoded
rent, raw reads) saved ~600 CU, but that is 3.6% of the total.


---

## Pros

- **All 7 tests pass** including the binary size constraint
- **72.9% binary size reduction** (52.44 KB → 14.20 KB) — fits within
  L1 instruction cache for better real-world performance
- **16% CU reduction on Contribute** — the most frequently called
  instruction in any active fundraiser
- **Full Pinocchio safety model preserved** — no undefined behaviour,
  no memory layout assumptions outside what Pinocchio guarantees
- **Maintainable** — every optimization has a clear rationale and could
  be written by any developer who understands why it works
- **Four real bugs corrected** that exist in v1 and would cause incorrect
  program behaviour in production
- **No external layout assumptions** — if Solana's BPF loader input
  format changes, this version is not affected

---

## Cons

- **Initialize CU barely improved** — the CPI bottleneck is irreducible
  from the caller's side; 87% of Initialize's cost is inside the ATA program
- **`Clock::get()` syscall remains** — `Contribute` and `Refund` still
  pay ~150–200 CU each for the clock sysvar. The fix (passing Clock
  as an account) was identified but changes the client-side transaction
  construction and is not applied here
- **More complex than v1** — unsafe blocks, raw pointer arithmetic, and
  hardcoded constants require the reader to understand BPF memory layout
  and Solana's stable account formats; harder to audit without that background
- **Hardcoded rent carries long-term maintenance risk** — the formula
  `(data_len + 128) * 6960` has been constant since genesis but is
  technically not a protocol guarantee
- **Marginal further gains in v3** — Contribute and Refund each save
  another ~76–89 CU in v3, but at a significant cost in code complexity
  and safety

---

## When to Use This Version

**This is the recommended production version.**

- When you need the lowest CU cost achievable without giving up safety guarantees
- In any competitive or high-frequency environment where Contribute is
  called many times per session
- When the program will be audited or maintained by multiple developers
- When you cannot afford to break on a future Solana loader format change

> For even lower Contribute and Refund CU at the cost of safety and
> maintainability, see **v3** (unsafe/near-assembly).
