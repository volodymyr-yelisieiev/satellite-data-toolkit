# Satellite Data Toolkit Pro — Rust Desktop App

Профессиональная версия приложения на Rust для Windows.

## Что добавлено

- NASA POWER как публичный API без ключа.
- Отдельный слой API Slots: Public / Protected / Proxy.
- EUMETSAT key + secret через OS keychain.
- NREL PVWatts key slot.
- Custom Weather Proxy slot.
- Cache layer.
- Live request logs.
- Request inspector.
- Source provenance ID.
- Rate-limit monitor.
- Export CSV / JSON.
- Отдельная кнопка Load Example, чтобы NASA не выглядела «подключённой заранее».

## Как запустить на Windows

1. Распакуй архив.
2. Дважды нажми `RUN_WINDOWS_BUILD_AND_START.cmd`.
3. Скрипт сам скачает Rust toolchain, соберёт release-версию и запустит приложение.

Первый запуск долгий, потому что Rust скачивает библиотеки и компилирует их. После этого приложение запускается из:

```text
satellite_data_toolkit_rust/target/release/satellite_data_toolkit.exe
```

## Важное ограничение

В этом архиве исходный код и Windows-загрузчик. Полностью готовый `.exe` нужно собирать на Windows, потому что в текущей среде нет Rust toolchain и Windows toolchain.

## NASA POWER

NASA POWER не требует API key. Поэтому в API Slots он отображается как:

```text
Public API — no key required
```

## EUMETSAT

EUMETSAT требует consumer key и consumer secret. Они сохраняются через OS keychain.
