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
| [`docs/SZKOLENIE-SIEC.md`](docs/SZKOLENIE-SIEC.md) | reverse proxy, CGNAT Netii, ASUS, NFC |
| [`docs/IPV6.md`](docs/IPV6.md) | test IPv6 na tym hoście; dziś tylko link-local |
| [`LICENSE`](LICENSE) | Apache-2.0 jako **tymczasowy** default laboratoryjny (decyzja prawna otwarta) |

## Stan

v0.0.3 + identity + Observer + Service Access (`echo` po policy) + Nitrokey OpenPGP jako HumanIdentity. `READY` dopiero po `ACCESS_GRANT`.

```text
cargo test --workspace

# sam HELLO (bez tożsamości)
cargo run -p mnp-server
cargo run -p mnp-client

# mutual auth + PIN/dotyk (po HELLO, gdy są już *.id)
./scripts/lab-server-auth    # terminal 1
./scripts/lab-client-auth    # terminal 2

# czy jest GUA/ULA (poza ::1)
./scripts/check-ipv6
```

## Co tu nie mieszka

Forge (`crates/forge-*`, kontrakt V2.5) zostaje obok, w korzeniu tego worktree. MNP nie wchodzi do workspace Cargo Forge.
