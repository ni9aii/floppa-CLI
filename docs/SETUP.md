# Установка и использование floppa-cli

Этот гайд — дополнение к [README.md](../README.md) с деталями установки и
повседневного использования CLI-only клиента.

## 1. Установка бинаря

### Из релиза

Зайдите на https://github.com/ni9aii/floppa-CLI/releases, скачайте бинарь
под свою архитектуру и установите в `PATH`:

```sh
# x86_64
sudo curl -L -o /usr/local/bin/floppa-cli \
  https://github.com/ni9aii/floppa-CLI/releases/latest/download/floppa-cli-linux-x86_64
sudo chmod +x /usr/local/bin/floppa-cli

# aarch64 (например, Raspberry Pi 4/5)
sudo curl -L -o /usr/local/bin/floppa-cli \
  https://github.com/ni9aii/floppa-CLI/releases/latest/download/floppa-cli-linux-aarch64
sudo chmod +x /usr/local/bin/floppa-cli
```

Проверка:

```sh
floppa-cli --help
```

### Из исходников

```sh
git clone https://github.com/ni9aii/floppa-CLI
cd floppa-CLI
cargo build --release --manifest-path floppa-cli/Cargo.toml
sudo cp floppa-cli/target/release/floppa-cli /usr/local/bin/
```

Зависимости для сборки: `pkg-config`, `libssl-dev` (Debian/Ubuntu) или
`openssl-devel` (RPM-based).

## 2. Аутентификация

```sh
sudo floppa-cli login
```

Откроется браузер с Telegram OAuth. После подтверждения токен сохраняется в
`~/.config/floppa-cli/` (или системный каталог; файл device.json с
идентификатором устройства создаётся автоматически).

Выйти из аккаунта:

```sh
sudo floppa-cli logout
```

## 3. Подключение

```sh
# WireGuard (по умолчанию)
sudo floppa-cli connect

# AmneziaWG (DPI-стойкий)
sudo floppa-cli connect --protocol amneziawg

# VLESS+REALITY
sudo floppa-cli connect --protocol vless
```

Дополнительные флаги `connect`:

| Флаг | Значение |
|------|----------|
| `--protocol <wireguard\|amneziawg\|vless>` | протокол (по умолчанию `wireguard`) |
| `--config <FILE>` | готовый `.conf` / VLESS-URI файл вместо запроса к API |
| `--interface <NAME>` | имя TUN-интерфейса (по умолчанию `floppa`) |
| `--no-dns` | не менять `/etc/resolv.conf` |
| `--api-url <URL>` | переопределить адрес сервера |

IPv6-маршруты обрабатываются автоматически: если в конфиге есть IPv6-роут,
клиент сам добавляет флаг `-6` к `ip route`. При отключении все добавленные
маршруты (и TUN-интерфейс, включая устаревший `floppa0`) удаляются — сеть
возвращается в исходное состояние.

### Проверка «живости» туннеля

В процессе работы клиент сам следит за туннелем (см. [RECONNECT.md](RECONNECT.md)):

- WireGuard/AmneziaWG — читает время последнего handshake через kernel UAPI;
  если handshake устарел (туннель «дохлый»), туннель пересобирается.
- VLESS+REALITY — делает TCP-connect пробу до эндпоинта; при недоступности —
  пересборка.

Это работает поверх systemd-реконнекта: внутренний watchdog ловит
транзитные обрывы, а `Restart=on-failure` (в юните) переподнимает клиент при
фатальной ошибке.


Посмотреть сгенерированный конфиг без подключения:

```sh
floppa-cli config --protocol wireguard
floppa-cli config --protocol vless
```

Список peer'ов аккаунта:

```sh
floppa-cli peers
```

## 4. Запуск как systemd-сервиса

```sh
sudo floppa-cli service install
```

Это:
1. сгенерирует `/etc/systemd/system/floppa-cli.service`, указывающий на текущий
   бинарь и выбранные аргументы подключения;
2. выполнит `systemctl daemon-reload`;
3. выполнит `systemctl enable --now floppa-cli` (включит и запустит).

Остановить и выключить:

```sh
sudo floppa-cli service uninstall
```

Посмотреть сгенерированный юнит без установки:

```sh
floppa-cli service print --config /etc/floppa-cli/client.conf --protocol wireguard
```

Юнит содержит `Restart=on-failure` — при падении клиента systemd его
переподнимет. См. также [RECONNECT.md](RECONNECT.md) про внутренний реконнект.

## 5. Логи и отладка

```sh
sudo floppa-cli connect --log-file /tmp/floppa-cli.log
```

Или через journald (если запущен как сервис):

```sh
journalctl -u floppa-cli -f
```

## 6. Известные ограничения

- Только Linux (WireGuard/AmneziaWG управляются через netlink; VLESS — через
  TUN + socks-обёртку).
- Требуются права root для сетевых операций.
- Клиент не разворачивает бэкенд — нужен внешний сервер Floppa VPN.

## 7. Обновление

Скачайте новый бинарь из релиза и замените `/usr/local/bin/floppa-cli`.
Если используется systemd-сервис — перезапустите:

```sh
sudo floppa-cli service uninstall
sudo floppa-cli service install
```
