# Szkolenie: odwrotne proxy, CGNAT Netii, ASUS, NFC

Notatka z rozmowy (lab MNP). Nie jest konfiguracją routera. Nic tu nie otwieramy na świat.

**Potwierdzone:** „cgrant” było pomyłką. Chodzi o **CGNAT Netii**, nie o host. Nie ma w tej sieci maszyny o nazwie cgrant.

---

## 1. Odwrotne proxy — krótkie szkolenie

Zwykły (forward) proxy: **Ty** wychodzisz do internetu *przez* pośrednika (firma, Tor, „proxy w przeglądarce”).

**Odwrotne proxy (reverse proxy):** pośrednik stoi **przed usługą** i to świat (albo inny Twój sprzęt) puka do pośrednika, a on **dopiero wtedy** sięga po prawdziwy serwer.

```text
telefon / MacBook
        │
        │  widzi tylko drzwi
        ▼
   odwrotne proxy          ← tu decyzja: kto, dokąd, czy w ogóle
        │
        │  już w środku, lokalnie
        ▼
   sshd / strona / echo
```

Czego **nie** robi: nie daje Ci całej obcej sieci jak VPN. Nie jest WireGuardiem.

Czego **robi**: jedna dziura (albo jedno połączenie), jedna usługa, ewentualnie „najpierw udowodnij kim jesteś”.

**MNP Service Access** ma być w tym duchu: QUIC + tożsamość (Nitrokey) + `GRANT`, potem strumień DATA = bajty SSH/RDP. `sshd` zostaje `sshd`. MNP nie implementuje pulpitu.

Lab dziś: odwrotne proxy do **echo** na localhost. Jeszcze nie do SSH i jeszcze nie przez NAT.

Kuzyni ze świata: nginx/Caddy przed stroną, Cloudflare Tunnel, Tailscale Serve. Tailscale jest mesh + relay; MNP celuje w węższy „wpuszczam tę usługę”, nie „sklejam LANy”.

---

## 2. Jak może wyglądać CGNAT u Netii

CGNAT = **NAT operatora**. Za mało IPv4 na świecie, więc Netia (i wielu ISP) sadza wielu klientów za **wspólnym** publicznym IP.

```text
Internet
    │
    │  jeden publiczny IPv4 współdzielony przez wielu klientów
    ▼
maszyna CGNAT Netii     ← tego nie konfigurowałeś i nie zobaczysz w ASUS-ie
    │
    │  często adres z puli 100.64.0.0/10  (RFC 6598, „shared”)
    │  albo inny prywatny
    ▼
router Netii (u Ciebie w szafce / od operatora)
    │  DMZ → ASUS
    ▼
router ASUS             LAN np. 192.168.50.0
    │
    ▼
CachyOS (kabel)         mnp-client / kiedyś mnp-server
```

**Jak to poznać (w praktyce):** WAN ASUS-a to nie „prawdziwy” publiczny IP, tylko coś z `100.64…` / `10…` / `100.x`. Panel Netii może mówić o CGNAT. Forward portu na ASUS-ie **nie** wystawia Cię na internet, bo dziura NATu jest **wyżej**, u operatora. DMZ Netia→ASUS też nie magicznie zdejmuje CGNAT — najwyżej ASUS dostaje wszystko, co *w ogóle* doleci z warstwy operatora.

Netia sama opisuje CGNAT jako współdzielenie publicznego IPv4; ograniczenie: **z internetu nikt nie zaczyna połączenia do Ciebie**, Ty możesz wychodzić.

Dlatego:

- **WireGuard jako serwer w domu** — ból. Nikt z zewnątrz nie trafi UDP 51820 na „Twój” IP, bo to nie jest tylko Twój IP.
- **Tailscale** — wychodzi **od Ciebie** do ich koordynacji/relay (DERP). Wychodzące CGNAT lubi. Dwa pudła za NAT-em spotykają się na czyimś serwerze, czasem potem przebijają NAT. Nie musisz otwierać portów na ASUS-ie. Stąd „jakoś sobie radzi”.
- **MNP nasłuchujące na CachyOS i czekające na telefon z LTE** — ten sam problem co WG, dopóki nie ma: IPv6, wyjścia *od domu* do miejsca z publicznym IP, albo relay.

To nie jest Twinja wina ani „zła konfiguracja CGNAT”. Tego się nie klika w ASUS-ie. Albo operator da publiczny IPv4 / IPv6, albo obchodzisz (Tailscale, VPS, wychodzący tunel).

---

## 3. Wychodzące z CachyOS przez ASUS — czy Netia to powiesi?

**Zazwyczaj nie.** CGNAT i ASUS są zbudowane tak, że **Ty wychodzisz, odpowiedź wraca**. Przeglądarka, `ssh user@vps`, aktualizacje — to działa.

```text
CachyOS  →  ASUS  →  Netia/CGNAT  →  internet  →  VPS / drugi peer
         wychodzące, stan NATu pamięta „to nasz klient”
```

Netia **nie musi nic wieszać** na ASUS-ie, żebyś *wychodził*. Wiesza, gdy chcesz **wejścia**: port forward, „serwer w domu”, inbound WireGuard.

Wniosek dla MNP:

- **CachyOS jako klient** (łączy się do peera, który ma publiczny IP / IPv6 / już otwarte połączenie) — przez ASUS powinno przejść jak Tailscale.
- **CachyOS jako serwer czekający na świat** — sam ASUS nie wystarczy przy CGNAT. Forward UDP na ASUS-ie jest potrzebny dopiero *wewnątrz* domu (telefon w Wi-Fi ASUS-a → CachyOS). Z LTE / z internetu i tak stopuje CGNAT.

Kiedyś zadanie „ustaw ASUS” = wąski forward UDP tylko dla MNP **w LAN-ie**, nie DMZ na cały świat i nie złudzenie, że to przebije Netię.

---

## 4. NFC — w telefonie czy osobny gadżet?

Dwie różne rzeczy:

**A. Token z NFC (to miałem na myśli)**  
Drugi Nitrokey / YubiKey **z anteną NFC** — nadal kawałek plastiku na breloku. Telefon ma **czytnik** NFC (cewka z tyłu obudowy). Przykładasz token do telefonu → telefon mówi „jest użytkownik”. To **nie** jest „NFC w ustawieniach Androida” jako tożsamość MNP. Klucz prywatny zostaje w tokenie, jak przy USB.

Na MacBooku ten sam token: USB-C albo NFC (jeśli laptop czyta NFC — wiele nie czyta, wtedy USB).

**B. Passkey / klucz w telefonie**  
Telefon sam jest tokenem (Secure Enclave, biometria). Inny model, inna kasa, inna utrata („zgubiłem telefon” = zgubiłem ten klucz). To nie jest Nitrokey.

Na start MNP: **A** — dwa fizyczne tokeny w allowliście („brelok USB”, „brelok NFC do telefonu”). Telefon bez drugiego plastiku nie podpisze HumanIdentity tak, jak dziś GPG na CachyOS.

UX z telefonu: przyłóż NFC **raz** na sesję + timeout (jak sudo), nie przy każdym `ls` przez SSH.

---

## 5. Potencjał MNP na *tej* sieci (LAN ASUS + Netia/CGNAT)

MNP **nie wygra** z Tailscale’em w „dwa NATy mają się polubić”. Tailscale ma relay; MNP go nie ma i nie powinien udawać VPN-u (KD 18).

Gdzie **ma** potencjał u Ciebie:

1. **LAN ASUS (kabel/Wi-Fi)** — telefon/Mac → CachyOS. CGNAT nie istnieje. Nitrokey + `GRANT ssh` to drzwi do usługi, nie tunel całej siatki. Lab `[::1]` jest prototypem właśnie tego, tylko na jednym pudle.
2. **Tożsamość człowieka na żelazie** — Tailscale klei urządzenia; MNP klei *Major + ten laptop + ta usługa*. Drugi token NFC = drugi wpis człowieka, nie magiczny mesh.
3. **IPv6, jeśli Netia kiedyś da** — Architecture v1 jest IPv6-only nie z fanaberii. CGNAT to choroba IPv4. Dwa globalne IPv6 = telefon LTE → dom bez forwardu na ASUS-ie.
4. **Wychodzący klient z CachyOS** — przez ASUS i CGNAT, jak przeglądarka. Peer musi być osiągalny (VPS / IPv6 / ktoś kto już słucha). Netia tego nie wiesza.

Gdzie **nie** ma potencjału bez zmiany gry:

- `mnp-server` w domu + telefon na LTE przez IPv4 = ten sam mur co WireGuard. Forward na ASUS-ie nie przebije CGNAT.
- Zamiana Tailscale na MNP „żeby wszędzie działało”. To inny produkt.

Słodki punkt: **domowy access z tożsamością**, Tailscale (albo IPv6) na „jestem poza domem”, MNP nie udaje overlayu.

## 6. Jak to spiąć z MNP (kolejność, bez roboty teraz)

1. Lab zostaje na localhost — świadomie, CGNAT nie obchodzi `[::1]`.
2. Reverse-proxy mental model: `GRANT ssh` → TCP do `sshd`, nie tunel LAN.
3. Żeby dojść z telefonu (LTE) do domu za CGNAT: albo IPv6, albo **wychodzący** klient z domu do czegoś publicznego, albo świadomy relay — nie „otwórz port na ASUS i będzie internet”.
4. ASUS później: tylko LAN. Netia/CGNAT osobna rozmowa z operatorem (publiczny IP / IPv6) albo obchodzenie.
5. Dwa tokeny + timeout sesji, zanim UX z telefonu ma sens.
6. Rotacja: CGNAT nie zmienia tego — Human na tokenie rzadko; Device/peer kalendarzowo; sesja krótka.

---

## Szybki słownik

| Hasło | Co to jest |
|--------|------------|
| NAT | Wiele urządzeń za jednym IP (Twój ASUS). |
| CGNAT | To samo, ale **u operatora**, wielu klientów za jednym publicznym IPv4. |
| DMZ (Netia→ASUS) | „Wszystko co doleci, idź na ASUS”. Nie zdejmuje CGNAT. |
| Reverse proxy | Drzwi przed jedną usługą, nie VPN. |
| Tailscale DERP | Relé, gdy dwa NATy nie umieją się przebić. |
| Token NFC | Plastik z anteną + czytnik w telefonie, nie „funkcja NFC” jako klucz. |
