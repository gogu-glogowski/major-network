# Major Network Protocol / Major Network Stack (MNP)

Folder roboczy **Major Network Protocol**. To **nie** jest część Forge V2.5.

Nazwa z Architecture v1: `major-network` (zamiast `Major_Network_Proto`). Identyfikatory: `MNP`, crate’y później `mnp-core` / `mnp-client` / `mnp-server`.

## Źródło

Zamknięte decyzje z czatu [Major's Protocol](https://chatgpt.com/share/6aa31c30-0d00-83eb-b4f0-1633b01a20bd).

## Dokumenty

| Plik | Co to jest |
|------|------------|
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Architecture v1 — stack, identity, non-goals, Key Decisions, PR Plan |
| [`docs/PROTOCOL.md`](docs/PROTOCOL.md) | kontrakt laboratoryjny ramek |
| [`docs/IDENTITY.md`](docs/IDENTITY.md) | software mutual auth: Ed25519, sekwencja, TLS exporter |
| [`LICENSE`](LICENSE) | Apache-2.0 jako **tymczasowy** default laboratoryjny (decyzja prawna otwarta) |

## Stan

v0.0.3 + identity lab: ramka `MNP1`, maszyna stanów, software Ed25519 mutual auth (fail closed gdy podasz plik trust). Bez 4. argumentu zostaje `LAB_AUTO_ACCEPT` (nie identity).

```text
cargo test --workspace

# dwa procesy, sam HELLO (LAB_AUTO_ACCEPT)
cargo run -p mnp-server
cargo run -p mnp-client

# mutual auth: najpierw zapisz announce serwera, potem klienta z trust
# (szczegóły: docs/IDENTITY.md)
```

Następny krok: Nitrokey jako backend HumanIdentity (po researchu API).

## Co tu nie mieszka

Forge (`crates/forge-*`, kontrakt V2.5) zostaje obok, w korzeniu tego worktree. MNP nie wchodzi do workspace Cargo Forge.
