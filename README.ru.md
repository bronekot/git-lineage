# git-lineage

[![CI](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml/badge.svg)](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml)
[![Лицензия: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[English](README.md) · Русский

`git-lineage` — кроссплатформенная консольная утилита на Rust для построения компактного семантического дерева веток Git. Вместо полного графа коммитов она показывает логических родителей, опережение и отставание веток, расхождения с upstream-ветками, сравнения соседних веток и совпадения с ветками удалённого репозитория.

После переключения командой `git lineage language ru` отчёт выглядит так:

```text
Данные из origin получены; граф веток:

main
├─ dev (текущая)                 +8 от main
│  └─ origin/dev                 +1 от dev
│     └─ commands-fun            +12 от origin/dev
└─ web-ui                        +13 от main
```

## Установка

Из Git-репозитория:

```console
cargo install --git https://github.com/bronekot/git-lineage
```

После первой публикации на crates.io:

```console
cargo install git-lineage
```

Cargo устанавливает исполняемый файл `git-lineage`. Git автоматически делает файлы с именем `git-*` внешними подкомандами:

```console
git lineage
```

К GitHub-релизам с тегами прикладываются готовые архивы для Windows, Linux и macOS.

## Использование

По умолчанию перед построением отчёта выполняется `git fetch --prune --quiet origin`. Для работы только с локально сохранёнными ссылками:

```console
git lineage --no-fetch
git lineage --offline
```

Заголовок отчёта сообщает, были ли получены данные из удалённого репозитория, пропущен ли `git fetch` или после его ошибки использованы сохранённые данные.

Основные команды:

| Команда | Назначение |
| --- | --- |
| `git lineage` | Построить отчёт |
| `git lineage init` | Создать `.git-lineage.toml` |
| `git lineage discover` | Показать предполагаемых родителей веток |
| `git lineage discover --write` | Сохранить предполагаемых родителей веток |
| `git lineage parent <CHILD> <PARENT>` | Установить логического родителя |
| `git lineage parent <CHILD> --unset` | Удалить логического родителя |
| `git lineage check` | Проверить ссылки, отношения предков, доступность remote и циклы |
| `git lineage language [en|ru]` | Показать или сохранить язык интерфейса |

Полезные параметры отчёта:

```console
git lineage --summary none|brief|full
git lineage --format text|json
git lineage --color auto|always|never
git lineage --ascii
git lineage --all-remotes
git lineage --show-synced-remotes
```

## Язык

По умолчанию интерфейс работает на английском. Посмотреть текущий язык или сохранить другой:

```console
git lineage language
git lineage language ru
git lineage language en
```

Выбор хранится в глобальной конфигурации Git под ключом `git-lineage.language`, поэтому действует во всех репозиториях и сохраняется после перезапуска терминала. Также доступен короткий псевдоним `lang`. Имена полей и типов пояснений в JSON не зависят от языка; человекочитаемые предупреждения переводятся.

## Конфигурация

Файл `.git-lineage.toml` необязателен. Явно заданные родители всегда имеют приоритет над автоматическим определением.

```toml
version = 1
root = "main"
remote = "origin"
fetch = true

[parents]
dev = "main"
feature = "origin/dev"

[display]
collapse_synced_upstreams = true
show_remote_only = false
summary = "brief"
```

Если короткое имя неоднозначно, используйте полное имя, например `refs/heads/dev`.

## Безопасность

`git-lineage` вызывает системный Git напрямую, передаёт аргументы отдельно и устанавливает `GIT_TERMINAL_PROMPT=0`. Утилита не запускает командную оболочку и не выполняет `merge`, `rebase`, `reset`, `cherry-pick` или `push`. Необязательный `git fetch` изменяет только ссылки удалённых веток.

Если `git fetch` не удался, но сохранённые ссылки удалённых веток доступны, построение отчёта продолжается с явным предупреждением. В JSON-режиме `stdout` остаётся машиночитаемым, а диагностика отправляется в `stderr`.

## Поддерживаемые платформы и инструменты

- Windows, Linux и macOS
- Rust 1.85 или новее
- Системный Git, доступный через `PATH`

## Участие в разработке

Правила находятся в [CONTRIBUTING.md](CONTRIBUTING.md) и [Code of Conduct](CODE_OF_CONDUCT.md), процесс релиза — в [RELEASING.md](RELEASING.md), сообщения об уязвимостях — в [SECURITY.md](SECURITY.md).

## Лицензия

Проект распространяется по [лицензии MIT](LICENSE).
