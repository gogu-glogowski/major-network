# Major Network Protocol / Major Network Stack (MNP)

Folder roboczy **Major Network Protocol**. To **nie** jest część Forge V2.5.

Nazwa z Architecture v1: `major-network` (zamiast `Major_Network_Proto`). Identyfikatory: `MNP`, crate’y później `mnp-core` / `mnp-client` / `mnp-server`.

## Źródło

Zamknięte decyzje z czatu [Major's Protocol](https://chatgpt.com/share/6aa31c30-0d00-83eb-b4f0-1633b01a20bd).

## Dokumenty

| Plik | Co to jest |
|------|------------|
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Architecture v1 — stack, identity, non-goals, Key Decisions, PR Plan |
| [`docs/PROTOCOL.md`](docs/PROTOCOL.md) | kontrakt laboratoryjny v0.0.1 (HELLO) i throwaway stałe v0.0.2 |
| [`LICENSE`](LICENSE) | Apache-2.0 jako **tymczasowy** default laboratoryjny (decyzja prawna otwarta) |

## Stan

v0.0.3: ramka `MNP1` + maszyna stanów z `LAB_AUTO_ACCEPT` (READY po HELLO, bez krypto). QUIC/TLS 1.3, wyłącznie IPv6, pin certu. To **nie** jest identity.

```text
cargo test --workspace

# dwa procesy (localhost IPv6)
cargo run -p mnp-server
# inny terminal, ten sam katalog — pinuje mnp-lab-cert.der
cargo run -p mnp-client
```

Następny krok wg Architecture v1: specyfikacja identity (dokument, zanim kod proofu).

## Co tu nie mieszka

Forge (`crates/forge-*`, kontrakt V2.5) zostaje obok, w korzeniu tego worktree. MNP nie wchodzi do workspace Cargo Forge.
