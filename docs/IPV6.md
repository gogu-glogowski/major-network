# Test IPv6 (ten host, ten LAN)

MNP v1 jest **IPv6-only**. Lab na `[::1]` tego nie sprawdza — loopback zawsze ma `::1`.

## Stan na 2026-09-11 (CachyOS, kabel do ASUS)

| Co | Wynik |
|----|--------|
| IPv6 w kernelu | włączone |
| Adres globalny (GUA, `2xxx:`) | **brak** |
| Adres lokalny unikalny (ULA, `fdxx:`) | **brak** |
| Link-local | `fe80::34a3:94ca:a646:c966` na `enp10s0` |
| Domyślna trasa v6 | **brak** |
| `ping -6 2001:4860:4860::8888` | sieć niedostępna |
| DNS AAAA | działa (po IPv4) |
| IPv4 „na świat” | `93.159.7.60` — pula Netia, typowy **CGNAT** |

Wniosek: **nie ma IPv6 poza link-local.** MNP między dwoma pudłami w LAN-ie po v6 też nie poleci, dopóki ASUS nie rozda przynajmniej ULA albo GUA. Telefon LTE → dom po v6 — tym bardziej nie.

To nie jest bug MNP. Router nie ogłasza IPv6 (albo Netia go nie daje na WAN).

## Co ma się pojawić, żeby test był „zielony”

**Minimum na LAN ASUS (telefon Wi-Fi ↔ CachyOS), bez internetu v6:**

- adres `fd00::/8` (ULA) na `enp10s0`
- trasa w LAN-ie (często wystarczy prefix z RA)
- drugi host z adresem w tym samym prefiksie

**Żeby LTE / świat doszedł do domu:**

- GUA `2000::/3` + default via ASUS
- Netia musi dać IPv6 na WAN ASUS-a; sam click w LAN nic nie przebije CGNAT v4

## Jak testować (skrypt)

Z `~/major-network`:

```bash
./scripts/check-ipv6
```

Kod wyjścia: `0` = jest GUA albo ULA; `2` = tylko link-local (stan dzisiejszy); `1` = IPv6 wyłączone.

Po zmianie na ASUS-ie / u Netii odpalasz ten sam skrypt. Nie ruszamy konfiguracji routera z tego dokumentu.

## Gdzie klikać (Ty, nie agent)

Na **ASUS-ie**: IPv6 włączone; WAN = Native / DHCPv6 / SLAAC (jak Netia w ogóle daje); LAN = Server / RA, żeby klienci dostali prefix.

U **Netii**: w panelu albo na bramce — czy jest IPv6. Często w PL nie ma; wtedy zostaje ULA tylko w domu.

Nie włączamy IPv6 tuneli (6in4, Cloudflare WARP) jako „oszustwa” pod MNP v1 — to nie jest ścieżka z Architecture.

## MNP

- `[::1]:4433` — działa dziś (`./scripts/lab-*-auth`).
- Bind na `enp10s0` ULA/GUA — **po** zielonym `check-ipv6`.
- Nie dodajemy IPv4 do MNP żeby ominąć ten test.
