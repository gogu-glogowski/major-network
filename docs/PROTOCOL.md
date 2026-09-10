# MNP — PROTOCOL (lab)

**Nie jest kontraktem v1.** Stałe i sekwencje poniżej są throwaway, żeby dało się zbudować spike. Produkcyjny layout, ALPN, certyfikaty Quinn i identity zostają w [ARCHITECTURE.md](ARCHITECTURE.md) → Open Questions.

Źródło: Architecture v1, milestone v0.0.1–v0.0.2.

## v0.0.1 — surowy HELLO (obecny cel implementacji)

Warstwa: aplikacja nad QUIC. Jeszcze **nie** ma własnej ramki.

```text
mnp-client                         mnp-server
    |                                   |
    |---- QUIC + TLS 1.3 (IPv6) ------->|
    |<--- handshake --------------------|
    |                                   |
    |---- "HELLO MNP"  (jeden bi-di) -->|
    |<--- "HELLO ACK" ------------------|
```

| Wymagane | Zakazane |
|----------|----------|
| Rust, Tokio, Quinn | identity, Observer, biometria |
| QUIC, TLS 1.3 | własny framing, własne crypto |
| IPv6 + `IPV6_V6ONLY` | IPv4 / IPv4-mapped |
| jeden strumień bidirectional | `SkipServerVerification` jako „tożsamość” |
| 0-RTT wyłączone | |

### Bind

- Server: `[::1]:4433` (lab), socket IPv6-only.
- Client: `[::]:0`, IPv6-only, łączy się na `[::1]:4433`.
- Role v0.0.x: client **inicjuje**, server **akceptuje**.

### ALPN (throwaway)

```text
mnp-lab/0
```

Token produkcyjny: Open Question.

### Lab TLS (throwaway, nie precedens)

1. Serwer przy starcie mintuje self-signed cert (`rcgen` lub równoważnik).
2. `ServerConfig` używa wyłącznie tego materiału.
3. Klient **pinuje** ten cert / mini-CA (plik obok binarek albo stdout serwera).
4. Zakaz skip-verify na ścieżce mylonej z MNP identity.
5. 0-RTT off.
6. Provider rustls: **`aws-lc-rs`** (default Quinn 0.11). Lab-only.

To **nie** jest `HumanIdentity` / `DeviceIdentity` / `PeerIdentity`.

### Payload v0.0.1

Dokładne bajty ASCII, bez ramki:

```text
HELLO MNP
HELLO ACK
```

v0.0.1 **może** `finish()` send stream po wymianie (one-shot). Od v0.0.2 stream żyje do `GOODBYE` / teardown QUIC.

## v0.0.2 — ramka (zaimplementowana w `mnp-core::frame`)

Szkic z Architecture v1. Parser wyłącznie w przyszłym `mnp-core`.

```text
MAGIC     4 B    b"MNP1"
VERSION   1 B    0
TYPE      1 B
FLAGS     2 B    0
LENGTH    4 B    rozmiar PAYLOAD, little-endian
SESSION   8 B    little-endian; zawsze 0 przez v0.0.3
PAYLOAD   N B
```

Nagłówek: 20 B.

| TYPE | Nazwa |
|------|--------|
| `0x01` | `HELLO` |
| `0x02` | `HELLO_ACK` |
| `0x03` | `ERROR` |
| `0x04` | `GOODBYE` |

`LAB_MAX_PAYLOAD = 65536` (64 KiB). `LENGTH` powyżej → `ERROR` / disconnect, **bez** alokacji payloadu.

`SESSION = 0` — brak allocatora. Nie czwarte pryncypium.

Nie `finish()` po HELLO: pętla nagłówek + payload.

## Identity

Spec: [`IDENTITY.md`](IDENTITY.md). Typy ramek `0x10`–`0x12`. Fail closed, bez `LAB_AUTO_ACCEPT` na tej ścieżce.

## Stack (przypomnienie)

```text
Application (Observer później)
        ↓
MNP
        ↓
QUIC (Quinn) + TLS 1.3 / rustls
        ↓
UDP → IPv6 only
```

QUIC wozi. MNP mówi *co* i *kto ma prawo*. MNP nie reimplementuje congestion, loss recovery, TLS, migration.
