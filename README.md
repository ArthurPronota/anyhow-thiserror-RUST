# `anyhow` и `thiserror`: разбор твоего примера

Твой `o_097` — правильное разделение: `thiserror` для **библиотечной** ошибки `StoreError`, `anyhow` для **приложения** (`main`). Разберём, что здесь хорошо и что можно уточнить.

## Что демонстрирует код

```rust
#[derive(Error, Debug)]
enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing key: {0}")]
    Missing(String),
}

fn load(path: &str) -> Result<String, StoreError> { ... }

fn main() -> Result<(), anyhow::Error> {
    match load(path) {
        Ok(v) => { println!("{v}"); Ok(()) },
        Err(err) => Err(err.into()),   // StoreError → anyhow::Error
    }
}
```

- `StoreError` — **типизированная** ошибка с вариантами, которые вызывающий может матчить.
- `load` возвращает `Result<T, StoreError>` — библиотечный стиль.
- `main` возвращает `Result<(), anyhow::Error>` — приложение может использовать `anyhow`.
- `err.into()` — конвертация `StoreError` в `anyhow::Error` через `From` (anyhow реализует `From<E: Error>`).

## Что можно улучшить

### 1. `main` можно упростить

Сейчас:

```rust
match load(path) {
    Ok(v) => { println!("{v}"); Ok(()) },
    Err(err) => Err(err.into()),
}
```

Эквивалент, но короче:

```rust
fn main() -> anyhow::Result<()> {
    let v = load(path)?;   // ? сам конвертирует StoreError → anyhow::Error
    println!("{v}");
    Ok(())
}
```

`anyhow::Result<T>` — это псевдоним `Result<T, anyhow::Error>`. Оператор `?` автоматически конвертирует **любую** ошибку, реализующую `Error + Send + Sync + 'static`, в `anyhow::Error`. `StoreError` подходит.

### 2. Контекст через `.context()` / `.with_context()`

Главная сила `anyhow` — **контекст**. Сейчас при ошибке `io: No such file or directory` непонятно, **какой файл** не открылся. Добавь контекст:

```rust
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let path = "Cargo.toml";
    let v = load(path)
        .with_context(|| format!("failed to load {path}"))?;
    println!("{v}");
    Ok(())
}
```

Теперь при ошибке вывод будет:

```
Error: failed to load Cargo.toml

Caused by:
    io: No such file or directory (os error 2)
```

`anyhow` показывает **цепочку** — `source()` из `thiserror` работает.

### 3. `Missing` не используется в `load`

Вариант `Missing(String)` объявлен, но `load` его не создаёт. Это нормально для демонстрации, но в реальном коде要么 добавить `get`-функцию:

```rust
fn get<'a>(map: &'a HashMap<String, String>, key: &str) -> Result<&'a str, StoreError> {
    map.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| StoreError::Missing(key.to_string()))
}
```

### 4. `#[error("io: {0}")]` — можно информативнее

`{0}` — это `Display` от `io::Error`, например `"No such file or directory (os error 2)"`. Итог: `"io: No such file or directory (os error 2)"`. Можно добавить путь:

```rust
#[error("io error while reading {path}: {source}")]
Io {
    path: String,
    #[source]
    source: std::io::Error,
}
```

Но тогда теряется `#[from]` (нужен ручной `map_err`). Компромисс: оставить `#[from]` + контекст на уровне `anyhow` (как в п. 2).

## Правило выбора: уточнение

Твой конспект верен, но добавлю границы:

| Слой | Инструмент | Почему |
|------|------------|--------|
| **Библиотека** (публичный API) | `thiserror` + enum | вызывающий матчит варианты, не парсит строки |
| **Приложение** (`main`, handlers) | `anyhow` | не нужно матчить, нужен контекст и цепочка |
| **Смешанный слой** | `thiserror` для домена + `anyhow` на границе | типизация внутри, гибкость снаружи |
| **Прототип** | `anyhow` везде | быстрее, потом можно типизировать |

Важное уточнение: **`anyhow` не заменяет `thiserror`** — они дополняют друг друга. `thiserror` даёт **тип**, `anyhow` — **стирание типа + контекст**. В твоём коде это ровно так: `StoreError` — тип, `anyhow::Error` — обёртка на уровне `main`.

## Что теряется при `err.into()`

Когда `StoreError` конвертируется в `anyhow::Error`:

- **Тип теряется** — снаружи `anyhow::Error`, нельзя матчить `StoreError::Missing`.
- **`source()` сохраняется** — цепочка работает.
- **`Display` сохраняется** — сообщение из `#[error(...)]`.
- **Downcast возможен** — `err.downcast_ref::<StoreError>()` вернёт `Some`, если нужно достать исходный тип.

Пример downcast:

```rust
if let Some(store_err) = err.downcast_ref::<StoreError>() {
    match store_err {
        StoreError::Missing(k) => println!("key {k} missing"),
        StoreError::Io(e) => println!("io: {e}"),
    }
}
```

Это способ **вернуть типизацию** на уровне приложения, если нужно.

## Итог

- `thiserror` — для **библиотечных** ошибок: enum, `Display`, `source`, `From`.
- `anyhow` — для **приложения**: стирание типа, контекст, цепочка.
- `?` в `main` с `anyhow::Result` конвертирует **любую** ошибку автоматически — `match` не нужен.
- `.context()` / `.with_context()` — главная сила `anyhow`: добавляет «что делали», сохраняя «почему упало».
- `downcast_ref::<StoreError>()` — способ вернуть типизацию, если нужно матчить на уровне приложения.
- Правило: **enum внутри, `anyhow` снаружи**. Твой код — ровно этот паттерн, просто `main` можно сократить до `load(path)?` + `.context(...)`.
