# TXPOS Architecture

TXPOS is being developed as a sequence of small, buildable milestones. The
system is security-first: each subsystem must fail closed, prefer least
privilege, and expose narrow contracts that can be tested before integration.

## Milestone 0

Milestone 0 establishes the repository structure and core Rust crates:

- `bootloader`: boot information and measurement contracts
- `kernel`: initialization facade tying early services together
- `memory`: page and physical frame allocation primitives
- `scheduler`: fixed-capacity priority round-robin scheduler
- `security`: capability-based sandbox policy
- `crypto`: verifier contracts and safe byte utilities
- `txshield`: runtime integrity measurement log
- `txsentinel`: behavior analysis counters
- `txvault`: authenticated-encryption provider contract and record metadata
- `txfirewall`: packet policy and rate limiting
- `filesystem`: TXFS superblock, CRC32, and journal metadata
- `networking`: Ethernet and IPv4 parsers

## Boot Direction

The next milestone should add a UEFI executable for `x86_64-unknown-uefi`,
record a kernel measurement, construct `BootInfo`, and transfer control to the
kernel. QEMU is required for the boot smoke test.

