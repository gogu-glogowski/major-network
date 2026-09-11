# MNP Service Access (lab)

MNP **nie** jest RDP, SSH ani VPN. Pilnuje: *ten człowiek, z tego urządzenia, do tej usługi na tym peerze*. Istniejący SSH/RDP/VNC zostają sobą.

`AUTHENTICATED` = wiemy kim jesteś.  
`READY` = policy wpuściła **co najmniej jedną** usługę. To już nie jest no-op.

PIN + palec są przy `PROOF` (raz na połączenie). `ACCESS_REQUEST` w tej samej sesji **nie** pyta znowu o token.

## Usługi (lab)

| id | Nazwa | W labie |
|----|--------|---------|
| `1` | `echo` | tak — ping/pong na strumieniu DATA |
| `2` | `ssh` | nazwa zarezerwowana; brak proxy TCP w tym kroku |
| `3` | `rdp` | j.w. |
| `4` | `vnc` | j.w. |

Default policy: zaufany announce (plik trust) może **tylko `echo`**. Reszta = deny.

## Ramki (kontrolny strumień, po identity)

| TYPE | Nazwa | Payload |
|------|--------|---------|
| `0x30` | `ACCESS_REQUEST` | `service:u8` |
| `0x31` | `ACCESS_GRANT` | `service:u8` |
| `0x32` | `ACCESS_DENY` | `service:u8` + `reason:u8` |

Po `GRANT` klient otwiera **kolejny** bi-di (logiczny `DATA`, bez pinowania ID):

| TYPE | Nazwa |
|------|--------|
| `0x40` | `DATA` |

Echo: klient `DATA`=`ping`, serwer `DATA`=`pong`. Bez drugiego podpisu Nitrokey.
