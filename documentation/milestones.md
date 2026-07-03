# TXPOS Milestones

## Milestone 0: Buildable Foundation

Status: implemented in this workspace.

Goals:

- Create the project structure from `build.txt`.
- Add tested Rust crates for core contracts and primitive services.
- Keep all implementation in safe Rust.
- Verify the workspace builds with `cargo test --workspace`.

## Milestone 1: UEFI Boot Smoke Test

Goals:

- Install or enable `x86_64-unknown-uefi`.
- Add a UEFI bootloader executable.
- Produce a bootable disk image.
- Boot in QEMU.
- Pass measured boot information to the kernel.

## Milestone 2: Kernel Console and Interrupt Skeleton

Goals:

- Add serial logging.
- Add panic reporting.
- Add interrupt descriptor table setup.
- Add timer tick integration with the scheduler.

