# TXPOS

<p align="center">
  <img src="docs/logo.png" alt="TXPOS Logo" width="180">
</p>

<h1 align="center">TXPOS</h1>

<p align="center">
  <strong>Security First Operating System</strong>
</p>

<p align="center">
  A modern, security-first, open-source operating system built from scratch using Rust.
</p>

---

## 🚀 Overview

**TXPOS** is a next-generation operating system focused on **security, performance, privacy, and reliability**.

Unlike traditional operating systems, TXPOS is being designed with a security-first architecture from the ground up. The long-term vision is to provide a secure platform for desktops, servers, cloud infrastructure, AI workloads, and enterprise environments.

> **Current Development Status:** 🚧 Milestone 1

Current milestone includes:

* ✅ UEFI Boot Support
* ✅ Custom Rust Bootloader
* ✅ Bare-metal Rust Kernel
* ✅ Graphical Boot Screen
* ✅ Kernel Dashboard Interface
* 🚧 Core Kernel Development (In Progress)

---

# ✨ Vision

Our mission is to build an operating system where **security is the default—not an afterthought.**

Future versions of TXPOS aim to include:

* Security-first architecture
* Memory-safe kernel components
* Modern desktop environment
* High-performance scheduler
* Native package manager
* Secure application sandboxing
* Built-in firewall
* Full disk encryption
* Secure networking stack
* Driver framework
* AI-ready architecture
* Container & virtualization support
* Enterprise-grade security
* Developer SDK
* Cloud-ready deployment
* Cross-platform development tools

---

# 🦀 Built With

* Rust
* UEFI
* Cargo Workspace
* x86_64 Architecture

---

# 📦 Build

Clone the repository:

```bash
git clone https://github.com/TXP-KING/TXP_OS.git
cd TXPOS
```

Build the complete operating system image:

```bash
cargo run -p xtask -- build-release-image
```

---

# 📁 Output

After a successful build, the generated bootable images will be available in:

```text
dist/
├── txpos.iso
└── txpos.img
```

* **txpos.iso** — Bootable ISO image
* **txpos.img** — Bootable hard disk image

---

# ▶️ Running TXPOS

TXPOS can currently be tested using:

* QEMU
* Oracle VirtualBox
* Real UEFI-compatible hardware (experimental)

For setup instructions and troubleshooting, see:

```
run&use.txt
```

> Future releases will include a dedicated installation guide and official documentation.

---

# 📂 Project Status

Current Milestone

| Component           | Status     |
| ------------------- | ---------- |
| UEFI Bootloader     | ✅          |
| Kernel              | ✅          |
| Basic GUI           | ✅          |
| Memory Management   | 🚧         |
| File System         | 🚧         |
| Drivers             | 🚧         |
| Networking          | 🚧         |
| Package Manager     | 📅 Planned |
| Desktop Environment | 📅 Planned |

---

# 🤝 Contributing

Contributions are welcome.

If you'd like to help improve TXPOS:

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Open a Pull Request

Bug reports, feature requests, and security improvements are always appreciated.

---

# 🛡️ Security

Security is the primary goal of TXPOS.

If you discover a security vulnerability, please report it responsibly by opening a private security report or contacting the maintainers before public disclosure.

---

# 📜 Roadmap

* Bootloader
* Kernel
* Memory Manager
* Process Scheduler
* Virtual Memory
* File System
* Device Drivers
* Networking
* GUI Desktop
* Package Manager
* Application Framework
* AI Integration
* Cloud Support
* Stable Release (v1.0)

---

# 📄 License

This project is licensed under the **Apache License 2.0**.

See the **LICENSE** file for the full license text.

---

# ⭐ Support

If you find TXPOS interesting, please consider:

* ⭐ Starring the repository
* 🍴 Forking the project
* 🐛 Reporting bugs
* 💡 Suggesting new features
* 🤝 Contributing code

Every contribution helps move TXPOS closer to becoming a secure, modern operating system.

---

<p align="center">
<strong>TXPOS — Security First. Built for the Future.</strong>
</p>