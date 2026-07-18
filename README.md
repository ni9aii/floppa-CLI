<p align="center">
  <img src="branding/hack_floppy_banana1.png" width="200" alt="Floppa VPN" />
</p>

<h1 align="center">floppa-cli</h1>

<p align="center">
CLI-клиент для Floppa VPN — подключение к WireGuard / AmneziaWG / VLESS
туннелям с консоли, без GUI.
</p>

---

## Что это

`floppa-cli` — **CLI-only форк** [okhsunrog/floppa-vpn](https://github.com/okhsunrog/floppa-vpn).
В отличие от оригинала, здесь нет daemon/Telegram-бота/admin-панели/Tauri-клиента —
только консольный клиент, который подключается к **уже работающему** серверу
Floppa VPN по его HTTP API.

Клиент сам по себе **не поднимает сервер** и не разворачивает бэкенд. Ему нужен
доступный инстанс floppa-vpn (свой или публичный `https://floppa.okhsunrog.dev`)
и аккаунт на нём.

- Протоколы: WireGuard, AmneziaWG (DPI-стойкий WireGuard), VLESS+REALITY
- Авто-восстановление туннеля после сна/Wi-Fi-роуминга/обрыва (см. [docs/RECONNECT.md](docs/RECONNECT.md))
- Проверка «живости» туннеля:
  - WireGuard/AmneziaWG — чтение свежести handshake через kernel UAPI
    (пересборка, если handshake устарел)
  - VLESS+REALITY — TCP-connect проба до эндпоинта
- Поддержка IPv6-маршрутов (флаг `-6` добавляется автоматически)
- Полная очистка сети при отключении (отслеживание добавленных маршрутов
  и их удаление, включая stale-интерфейс `floppa0`)
- systemd-юнит для запуска коннектора как сервиса (`floppa-cli service install`)
- Только Linux (WireGuard управляется через netlink)

## История форка

Форк развивался независимо от оригинала: `v0.2.0-cli-alpha` → `v0.2.1`
(CLI-альфы) → `v0.3.0-cli` (первый стабильный релиз, без суффикса `-cli-alpha`).
Ветка `main` форка — рабочая база релизов; актуальный бэкенд подтягивается
точечной сверкой HTTP-API (см. [docs/RECONNECT.md](docs/RECONNECT.md)),
полный бэкенд/фронтенд в форк не включён намеренно.

## Требования

- Linux (x86_64 или aarch64)
- Права root — для поднятия TUN-интерфейса и записи маршрутов/DNS
- Аккаунт на сервере Floppa VPN (токен выдаётся через `floppa-cli login`)
- Утилиты: `ip`, `wg` (для WireGuard/AmneziaWG), `resolvectl` (при systemd-resolved)

## Установка

### Из релиза (рекомендуется)

Скачайте бинарь под свою архитектуру со страницы
[Releases](https://github.com/ni9aii/floppa-CLI/releases) и положите в `PATH`:

```sh
# пример для x86_64
sudo curl -L -o /usr/local/bin/floppa-cli \
  https://github.com/ni9aii/floppa-CLI/releases/latest/download/floppa-cli-linux-x86_64
sudo chmod +x /usr/local/bin/floppa-cli
```

Подробный гайд с примерами конфигов и systemd — в [docs/SETUP.md](docs/SETUP.md).

### Из исходников

```sh
git clone https://github.com/ni9aii/floppa-CLI
cd floppa-CLI
cargo build --release --manifest-path floppa-cli/Cargo.toml
sudo cp floppa-cli/target/release/floppa-cli /usr/local/bin/
```

> Нужны `libssl-dev` и `pkg-config` (для сборки TLS-зависимостей).

## Быстрый старт

```sh
# 1. Залогиниться (откроется браузер с Telegram OAuth)
sudo floppa-cli login

# 2. Подключиться (протокол по умолчанию — wireguard)
sudo floppa-cli connect

# или явно указать протокол:
sudo floppa-cli connect --protocol amneziawg
sudo floppa-cli connect --protocol vless
```

После `connect` клиент создаёт (или переиспользует) peer для этого устройства,
поднимает TUN-интерфейс и держит туннель, переподнимая его при обрывах.

## Команды

```
floppa-cli [OPTIONS] <COMMAND>

Commands:
  login    Log in via Telegram (opens browser)
  connect  Connect to VPN (auto-detects WireGuard/AmneziaWG .conf or VLESS URI)
  peers    List your peers
  config   Fetch and print config (WireGuard/AmneziaWG .conf or VLESS URI)
  logout   Remove saved login token
  service  Manage the systemd unit (install/uninstall the connector as a service)
  help     Print this message or the help of the given subcommand(s)
```

Полезные флаги:

- `--api-url <URL>` / переменная `FLOPPA_API_URL` — переопределить адрес сервера
  (по умолчанию `https://floppa.okhsunrog.dev/api`)
- `connect --config <FILE>` — подключиться по готовому `.conf` / VLESS-URI файлу
- `connect --no-dns` — не трогать `/etc/resolv.conf` (оставить системный DNS)
- `connect --interface <NAME>` — имя TUN-интерфейса (по умолчанию `floppa`)

### Запуск как сервис (systemd)

```sh
# установить юнит, включить и сразу запустить
sudo floppa-cli service install

# посмотреть, что сгенерирует юнит, без установки
floppa-cli service print --config /etc/floppa-cli/client.conf

# удалить
sudo floppa-cli service uninstall
```

Юнит перезапускает клиент при падении (`Restart=on-failure`), а внутри клиента
работает свой watchdog-реконнект (см. [docs/RECONNECT.md](docs/RECONNECT.md)).

## Авто-восстановление

Клиент следит за туннелем и пересобирает его при:

- выходе из сна (через systemd-logind `PrepareForSleep` на Linux),
- периодической проверке здоровья (каждые ~30 с: свежесть WireGuard handshake
  или доступность VLESS-эндпоинта),
- фатальной ошибке (тогда управление передаётся systemd `Restart=on-failure`).

Подробнее: [docs/RECONNECT.md](docs/RECONNECT.md).

## Отличия от оригинала

Этот репозиторий — **только CLI-клиент**. Из оригинального floppa-vpn здесь
присутствует лишь `floppa-cli/`. Бэкенд (daemon, server, Tauri-клиент, бот)
намеренно удалён: клиенту он не нужен, он работает поверх публичного или
вашего собственного сервера по HTTP API.

Релизы форка тегируются отдельно (без суффикса `-cli-alpha`), начиная с
`v0.3.0-cli`.

## Сборка и CI

- `ci.yml` проверяет `floppa-cli`: `cargo fmt --check`, `clippy -D warnings`,
  `cargo test`.
- `release.yml` собирает бинари `floppa-cli-linux-x86_64` и
  `floppa-cli-linux-aarch64` по тегу `v*`.

## Лицензия

MIT. См. [LICENSE](LICENSE).
