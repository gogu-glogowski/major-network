# MNP Identity Protocol (software, lab)

Zamyka **OQ 8** (prymitywy) i **OQ 10** (sekwencja) z Architecture v1 na potrzeby software-key mutual auth. **Nie** zamyka API Nitrokey, store’u produkcyjnego ani policy.

TLS 1.3 nadal wozi ruch. Ten protokół udowadnia *kto* jest na drugim końcu. Nie szyfruje QUIC ponownie.

## Co jest udowadniane

Trzy pryncypia, osobne klucze:

| Pryncypium | Lab software | Później |
|------------|--------------|---------|
| `HumanIdentity` | klucz Ed25519 (stand-in) | Nitrokey 3A Mini (PoP) |
| `DeviceIdentity` | klucz Ed25519 maszyny | ten sam model, inny klucz |
| `PeerIdentity` | klucz Ed25519 serwera/routera | ten sam model |

W topologii v0.0.x (client inicjuje, server akceptuje):

- klient dowodzi **Human + Device**
- serwer dowodzi **Peer + Device**

Serial / MAC / nazwa hosta **nie** są dowodem. Lab TLS cert **nie** jest dowodem tożsamości MNP.

Mutual auth: obie strony muszą zweryfikować obie pary kluczy drugiej strony. Brak dowodu ⇒ brak `AUTHENTICATED` / `READY` (fail closed). `LAB_AUTO_ACCEPT` nie jest defaultem tej ścieżki.

## Prymitywy (OQ 8)

| Element | Wybór | Dlaczego |
|---------|--------|----------|
| Podpis | **Ed25519** (RFC 8032) | sprawdzony, krótki, bez własnej krzywej |
| Nonce | 32 B z CSPRNG | anti-replay w ramach sesji |
| Wiązanie z transportem | **TLS 1.3 exporter** 32 B, label `EXPORTER-MNP-Identity`, context pusty | proof nie przenosi się na inną sesję QUIC |
| Hash transkryptu | SHA-256 tylko jako składanka pól (pola są już stałej długości; podpis idzie na kanoniczny bufor, nie na „własny szyfr”) | |
| AEAD / własna krzywa / MCE | **zakazane** na tej ścieżce | Kerckhoffs; MCE zostaje osobnym torem edukacyjnym |

Nitrokey później podpisuje ten sam transkrypt jako backend `HumanIdentity`. Nie zmienia sekwencji.

## Sekwencja (OQ 10)

Jeden strumień bi-di (nadal bez numerów stream ID). Po `HELLO` / `HELLO_ACK`:

```text
client                              server
  |-- HELLO ----------------------->|
  |<-- HELLO_ACK -------------------|
  |-- ID_ANNOUNCE ----------------->|   Human pk + Device pk
  |<-- ID_ANNOUNCE -----------------|   Peer pk + Device pk
  |-- CHALLENGE (nonce) ----------->|
  |<-- CHALLENGE (nonce) -----------|
  |-- PROOF (2 × Ed25519) --------->|
  |<-- PROOF (2 × Ed25519) ---------|
  |                                 |
  verify both                       verify both
  AUTHENTICATED → READY             AUTHENTICATED → READY
```

To scala skrót z czatu (`HELLO → CHALLENGE → SIGNED PROOF`) z wariantem `SERVER_ID` / `DEVICE_ID`: identyczności jadą jako `ID_ANNOUNCE`, nie jako osobna magia poza ramką.

Nowe `TYPE` w ramce v0.0.2 (payload laboratoryjny):

| TYPE | Nazwa | Payload |
|------|--------|---------|
| `0x10` | `ID_ANNOUNCE` | `kind:u8` + `device_pk:32` + `principal_pk:32` |
| `0x11` | `CHALLENGE` | `nonce:32` |
| `0x12` | `PROOF` | `device_sig:64` + `principal_sig:64` |

`kind`: `1` = human (klient), `3` = peer (serwer). Device nie ma osobnego kind — zawsze pole `device_pk`.

Exporter **nie** leci na przewodzie. Obie strony liczą go z sesji TLS. Jeśli się nie zgadza, weryfikacja podpisu pada.

## Transkrypt podpisu

Kanoniczny bufor, identyczny po obu stronach:

```text
b"MNP-IDENTITY-V0" || 0x00
|| client_announce (65 B)
|| server_announce (65 B)
|| client_nonce (32 B)
|| server_nonce (32 B)
|| exporter (32 B)
```

Dwa podpisy Ed25519 na **tym samym** buforze: klucz urządzenia i klucz pryncypium (human albo peer).

Weryfikacja:

1. `kind` zapowiedziany zgadza się z rolą (klient=1, serwer=3).
2. Para `(device_pk, principal_pk)` jest na liście zaufania (plik / pin labowy).
3. Oba podpisy przechodzą.
4. Nonce ma 32 B (brak cache między sesjami w labie — wystarczy wiązanie exporterem).

Fail closed: jakikolwiek krok nie przechodzi → disconnect, stan zostaje `IDENTITY_PENDING` albo wraca do `DISCONNECTED`.

## Trust lab (OQ 13 zostaje otwarte)

Nie projektujemy keystore’u. Lab:

- każda strona trzyma dwa klucze Ed25519 (device + principal)
- publiczny bundle 65 B jak `ID_ANNOUNCE`
- druga strona dostaje ten plik out-of-band (jak `mnp-lab-cert.der`)

TOFU / CA / rotacja — nie w tym dokumencie.

## Co świadomie zostaje otwarte

- API Nitrokey (PIV / OpenPGP / FIDO2)
- czy Human musi być w każdej sesji labowej po hardware (tu: tak, software stand-in)
- produkcyjny pin/PKI
- `SESSION` vs Connection ID (nadal 0)
- drugi strumień `IDENTITY` (nadal jeden bi-di)
