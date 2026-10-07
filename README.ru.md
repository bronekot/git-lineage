# git-lineage

[![CI](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml/badge.svg)](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/git-lineage.svg)](https://crates.io/crates/git-lineage) [![Лицензия: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[English](README.md) · Русский

`git-lineage` строит фактический граф включения историй всех локальных и удалённых ссылок на ветки. Утилита использует только историю Git: проектной конфигурации, назначенных и логических родителей нет.

Граф строится так:

1. Читаются все ссылки из `refs/heads` и `refs/remotes`.
2. Имена, указывающие на один коммит, объединяются.
3. Для каждой пары вычисляется число уникальных коммитов слева и справа — эквивалент `git rev-list --left-right --count A...B`.
4. Истории упорядочиваются по строгому включению, после чего удаляются транзитивные рёбра.
5. Отношения разошедшихся веток описываются отдельно. Поэтому у merge-ссылки может быть несколько непосредственных исторических предков.

```text
Фактический граф refs по локальным данным:

main (= origin/main)
└─ dev (= origin/dev, origin/player-fix)  +8 от main
   └─ commands-fun (текущая)              +12 от dev

Ссылок: 6, узлов: 3 (объединено совпадающих ссылок: 3), связей после транзитивного сокращения: 2.
```

## Установка

Установка последнего релиза с crates.io:

```console
cargo install git-lineage --locked
```

Обычно Cargo устанавливает программу в `~/.cargo/bin`, а в Windows — в `%USERPROFILE%\.cargo\bin`. Этот каталог должен находиться в `PATH`, чтобы Git обнаружил внешнюю команду `git-lineage`.

```console
git-lineage --version
git lineage --version
```

Текущую разрабатываемую версию 0.2 можно установить из ветки `dev`:

```console
cargo install --git https://github.com/bronekot/git-lineage --branch dev --locked
```

В разделе [GitHub Releases](https://github.com/bronekot/git-lineage/releases) доступны архивы для Windows x86_64, Linux x86_64 и macOS Apple Silicon.

## Использование

По умолчанию `git lineage` получает данные из `origin` и удаляет устаревшие remote-tracking refs, после чего читает все ссылки на ветки, сохранённые локально. Параметр `--remote` выбирает другой remote для получения данных, но не фильтрует граф: в нём всегда учитываются ссылки из всех remote.

```console
git lineage
git lineage --no-fetch
git lineage --offline
git lineage --remote upstream
```

| Команда | Назначение |
| --- | --- |
| `git lineage` | Построить фактический граф |
| `git lineage report` | Явно построить фактический граф |
| `git lineage language [LANGUAGE]` | Показать или сохранить язык интерфейса (`en` или `ru`) |

| Параметр | Действие |
| --- | --- |
| `--summary none` | Не выводить пояснения; предупреждения сохраняются |
| `--summary brief` | Пояснить tracking-отношения, удалённые upstream и расхождения текущей ветки с соседними (по умолчанию) |
| `--summary full` | Пояснить каждую пару разошедшихся refs |
| `--format json` | Вывести узлы, все отношения пар, сокращённые рёбра, пояснения и счётчики в JSON |
| `--color always` | Всегда использовать цвет; также доступны `auto` (учитывает `NO_COLOR`) и `never` |
| `--ascii` | Использовать символы дерева из ASCII |
| `--remote NAME` | Получить данные из `NAME` вместо `origin`; отображаемые refs не фильтруются |

В режиме brief число остальных разошедшихся пар выводится подсказкой, а в JSON — полем `hidden_diverged_pairs`.

Каждое отношение пары в JSON содержит счётчики `left_only` и `right_only`, а также значение `left_ancestor`, `right_ancestor` или `diverged`. Совпадающие указатели представлены как псевдонимы одного узла.

## Язык

По умолчанию используется английский. Выбор хранится глобально под ключом `git-lineage.language` и сохраняется после перезапуска терминала:

```console
git lineage language
git lineage language ru
git lineage language en
```

Имена полей JSON и значения `kind` не зависят от выбранного языка.

## Конфигурация

Файла `.git-lineage.toml` нет; файлы с таким именем игнорируются. Отчёт полностью вычисляется из refs и истории коммитов. Команды `init`, `discover`, `parent` и `check` в версии 0.2 отсутствуют.

## Безопасность

`git-lineage` передаёт системному Git отдельные аргументы и устанавливает `GIT_TERMINAL_PROMPT=0`. Утилита не просит Git выполнять `merge`, `rebase`, `reset`, `cherry-pick` или `push`.

Стандартный отчёт выполняет `git fetch --prune`. Git может обновлять и удалять remote-tracking refs, записывать `FETCH_HEAD`, получать теги и настроенные подмодули, а также запускать обслуживание. Используйте `--no-fetch`, чтобы исключить сетевой доступ и изменения refs. Если remote не настроен, отчёт без ошибки строится по локальным данным. Если получение данных завершится ошибкой, отчёт по возможности будет построен по сохранённым refs. В режиме JSON диагностика направляется в `stderr`, поэтому `stdout` остаётся корректным JSON.

## Поддерживаемые платформы и инструменты

- Windows, Linux и macOS
- Rust 1.85 или новее
- Системный Git, доступный через `PATH`

## Участие в разработке

См. [CONTRIBUTING.md](CONTRIBUTING.md), [Code of Conduct](CODE_OF_CONDUCT.md), [RELEASING.md](RELEASING.md) и [SECURITY.md](SECURITY.md).

## Лицензия

Проект распространяется по [лицензии MIT](LICENSE).
