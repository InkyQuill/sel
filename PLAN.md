# План реализации утилиты `sel`

## Обзор

Утилита `sel` — консольная утилита для извлечения фрагментов текстовых файлов по номерам строк, диапазонам, позициям или регулярным выражениям. Работает потоково, поддерживает контекст, подходит для больших файлов.

**Исправление**: Если селектор опущен (и нет `-e`), выводим весь файл с номерами строк (эмуляция `cat -n`).

---

## Этап 1: Базовая структура проекта

**Цель**: Создать минимально работающий прототип.

### Задачи

- [ ] Инициализация проекта `cargo init`
- [ ] Настройка `Cargo.toml` с зависимостями
- [ ] Базовая структура `src/`:
  ```
  src/
  ├── main.rs           # Точка входа, разбор аргументов
  ├── cli.rs            # Определение CLI через clap derive
  ├── selector.rs       # Парсинг селекторов (строки/диапазоны/позиции)
  ├── reader.rs         # Потоковое чтение файлов
  ├── output.rs         # Форматирование вывода
  ├── error.rs          # Типы ошибок
  └── lib.rs            # Библиотечная экспозиция (для тестов)
  ```

- [ ] Реализация базового CLI с `clap` derive:
  ```rust
  #[derive(Parser)]
  #[command(name = "sel")]
  #[command(about = "Select slices from text files", long_about = None)]
  struct Cli {
      /// Selector (line number, range, position, or omitted for all lines)
      selector: Option<String>,

      /// Show N lines of context before and after matches
      #[arg(short = 'c', long = "context", value_name = "N")]
      context: Option<usize>,

      /// Show N characters of context around position
      #[arg(short = 'n', long = "char-context", value_name = "N")]
      char_context: Option<usize>,

      /// Don't output line numbers
      #[arg(short = 'l', long = "no-line-numbers")]
      no_line_numbers: bool,

      /// Regular expression pattern (PCRE-like)
      #[arg(short = 'e', long = "regex", value_name = "PAT")]
      regex: Option<String>,

      /// Always print filename
      #[arg(short = 'H', long = "with-filename")]
      with_filename: bool,

      /// Color output [auto, always, never]
      #[arg(long = "color", value_name = "WHEN")]
      color: Option<String>,

      /// Input file(s)
      #[arg(value_name = "FILE")]
      files: Vec<PathBuf>,
  }
  ```

### Критерии завершения

- `cargo build` успешно собирается
- `sel --help` показывает справку
- `sel --version` показывает версию

---

## Этап 2: Парсер селекторов

**Цель**: Реализовать парсинг всех форматов селекторов.

### Задачи

- [ ] Определить перечисление селекторов:
  ```rust
  pub enum Selector {
      All,                    // Без селектора - все строки
      LineNumbers(Vec<LineSpec>),  // N, M-N, N1,N2,M1-M2
      Positions(Vec<Position>),    // L:C, L1:C1,L2:C2
  }

  pub enum LineSpec {
      Single(usize),     // N
      Range(usize, usize), // M-N
  }

  pub struct Position {
      pub line: usize,
      pub column: usize,  // в байтах, начиная с 1
  }
  ```

- [ ] Парсинг непозиционного селектора:
  - `"42"` → `Single(42)`
  - `"10-20"` → `Range(10, 20)`
  - `"1,5,10-15,20"` → `[Single(1), Single(5), Range(10, 15), Single(20)]`
  - Проверка: `M <= N` для диапазона
  - Проверка: номера > 0

- [ ] Парсинг позиционного селектора:
  - `"23:260"` → `Position { line: 23, column: 260 }`
  - `"15:30,23:260"` → две позиции
  - Проверка: колонки > 0

- [ ] Валидация смешивания:
  - Если есть `:` хотя бы в одном элементе → все должны иметь `:`
  - Иначе → ошибка

- [ ] Обработка пустого селектора:
  - `None` или `""` → `Selector::All`

### Тесты

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_line() {
        assert_eq!(parse_selector("42"), Ok(Selector::LineNumbers(vec![LineSpec::Single(42)])));
    }

    #[test]
    fn parse_range() {
        assert_eq!(parse_selector("10-20"), Ok(Selector::LineNumbers(vec![LineSpec::Range(10, 20)])));
    }

    #[test]
    fn parse_mixed() {
        let sel = parse_selector("1,5,10-15,20").unwrap();
        // ...
    }

    #[test]
    fn parse_position() {
        assert_eq!(parse_selector("23:260"), Ok(Selector::Positions(vec![Position::new(23, 260)])));
    }

    #[test]
    fn reject_mixed_selector() {
        assert!(parse_selector("1,23:260").is_err());
    }

    #[test]
    fn parse_empty_selector() {
        assert_eq!(parse_selector(""), Ok(Selector::All));
        assert_eq!(parse_selector(None), Ok(Selector::All));
    }
}
```

### Критерии завершения

- Все форматы селекторов парсятся корректно
- Ошибки валидации обрабатываются с понятными сообщениями
- Тесты покрывают все случаи

---

## Этап 3: Потоковое чтение и базовый вывод

**Цель**: Реализовать чтение файла и вывод строк по номерам.

### Задачи

- [ ] Чтение файла с `BufReader`:
  ```rust
  pub struct LineReader<R: Read> {
      reader: BufReader<R>,
      current_line: usize,
  }
  ```

- [ ] Итератор по строкам с номерами:
  ```rust
  pub struct LinesWithNumbers<R: Read> {
      reader: LineReader<R>,
  }

  impl<R: Read> Iterator for LinesWithNumbers<R> {
      type Item = io::Result<(usize, String)>;
      // ...
  }
  ```

- [ ] Фильтрация по номерам строк:
  - Для `Selector::LineNumbers` — проверка, входит ли номер в список
  - Оптимизация: сортировка и бинарьный поиск для больших списков
  - Объединение пересекающихся диапазонов

- [ ] Обработка `Selector::All`:
  - Вывод всех строк с номерами

- [ ] Базовый вывод:
  ```rust
  pub struct OutputFormatter {
      show_line_numbers: bool,
      show_filename: bool,
      filename: Option<String>,
      color: ColorMode,
  }
  ```

### Критерии завершения

- `sel 10-20 file.txt` выводит строки 10-20
- `sel 5,10,15 file.txt` выводит строки 5, 10, 15
- `sel file.txt` выводит весь файл с номерами строк
- Работа с большими файлами без загрузки в память

---

## Этап 4: Позиционные селекторы и символьный контекст

**Цель**: Реализовать опцию `-n` для позиционных селекторов.

### Задачи

- [ ] Парсинг позиционных селекторов (уже сделано в этапе 2)

- [ ] Реализация `-n` — символьного контекста:
  ```rust
  pub struct Fragment {
      pub line_number: usize,
      pub content: String,
      pub start_column: usize,  // начало фрагмента в строке
      pub target_column: usize, // целевая позиция
  }

  impl Fragment {
      pub fn new(line: &str, column: usize, context: usize) -> Self {
          let line_bytes = line.as_bytes();
          let line_len = line_bytes.len();

          let start = if column <= context + 1 {
              0
          } else {
              column - context - 1
          };
          let end = min(line_len, column + context);

          // ...
      }

      pub fn format(&self) -> String {
          // "23: Это пример строки..."
      }

      pub fn pointer_line(&self) -> String {
          // "       ^"
      }
  }
  ```

- [ ] Вывод с указателем:
  - Фрагмент строки
  - Строка с указателем `^` под целевой колонкой

- [ ] Обработка выхода за границы:
  - Колонка > длины строки → фрагмент до конца
  - Указатель в конец или не выводится

### Тесты

```rust
#[test]
fn test_char_context_middle() {
    let line = "Это пример строки с длинным текстом";
    let frag = Fragment::new(line, 10, 5);
    assert!(frag.content.contains("пример"));
}

#[test]
fn test_char_context_boundary() {
    let line = "short";
    let frag = Fragment::new(line, 100, 10);
    assert_eq!(frag.content, "short");
}
```

### Критерии завершения

- `sel -n 10 23:260 file.txt` работает корректно
- Указатель выводится в правильной позиции
- Граничные случаи обрабатываются

---

## Этап 5: Строчный контекст (`-c`)

**Цель**: Реализовать вывод контекста вокруг совпадений.

### Задачи

- [ ] Кольцевой буфер для строк контекста:
  ```rust
  pub struct ContextBuffer {
      buffer: VecDeque<Option<(usize, String)>>,
      capacity: usize,
      current_line: usize,
  }

  impl ContextBuffer {
      pub fn new(context_size: usize) -> Self {
          // Храним N строк "до" + текущую + N строк "после"
          // Но для поточного чтения: храним N строк "до"
          let capacity = context_size;
          // ...
      }

      pub fn push(&mut self, line_no: usize, line: String) {
          // ...
      }

      pub fn get_context(&self, target_line: usize) -> Vec<(usize, String, bool)> {
          // bool = true для целевой строки
      }
  }
  ```

- [ ] Алгоритм с одним проходом:
  1. Читаем файл построчно
  2. Для `Selector::LineNumbers`:
     - Если строка в списке → выводим контекст
     - Используем кольцевой буфер для строк "до"
     - Читаем N строк "после" при совпадении
  3. Объединение пересекающихся интервалов контекста

- [ ] Пометка целевых строк:
  - Символ `>` в начале (перед номером или цветом)

- [ ] Совместимость с `-n`:
  - Целевая строка выводится с фрагментом и указателем
  - Контекстные строки — полностью

### Оптимизация

Для больших файлов и множества селекторов:
- Сортировка и объединение диапазонов
- Предварительное вычисление интервалов вывода
- Слияние пересекающихся контекстов

### Критерии завершения

- `sel -c 3 42 file.txt` показывает 3 строки до и после
- Пересекающиеся контексты объединяются
- Целевые строки помечаются

---

## Этап 6: Режим регулярных выражений (`-e`)

**Цель**: Реализовать поиск по регулярным выражениям.

### Задачи

- [ ] Интеграция крейта `regex`:
  ```rust
  pub struct RegexMatcher {
      regex: Regex,
  }

  impl RegexMatcher {
      pub fn matches(&self, line: &str) -> bool {
          self.regex.is_match(line)
      }

      pub fn find(&self, line: &str) -> Option<(usize, usize)> {
          // (start, end) в байтах
          self.regex.find(line).map(|m| (m.start(), m.end()))
      }
  }
  ```

- [ ] Режим `-e` без `-n`:
  - Вывод полных строк, содержащих совпадение

- [ ] Режим `-e` с `-n`:
  - Фрагмент вокруг первого совпадения
  - Указатель под началом совпадения

- [ ] Поддержка нескольких файлов:
  - Формат `{filename}:{line}:{content}`
  - Опция `-H` для принудительного вывода имени файла

- [ ] Обработка ошибок компиляции regex:
  - Понятное сообщение об ошибке
  - Код возврата 1

### Критерии завершения

- `sel -e ERROR log.txt` ищет "ERROR"
- `sel -c 2 -e TODO source.rs` с контекстом
- `sel -e 'pattern' *.rs` по нескольким файлам

---

## Этап 7: Подавление номеров строк (`-l`) и форматирование

**Цель**: Реализовать опцию `-l` и finalize форматирование.

### Задачи

- [ ] Опция `-l`:
  - Подавление номеров строк в выводе
  - Сохранение имен файлов для режима `-e` с несколькими файлами

- [ ] Форматирование вывода:
  ```rust
  pub enum OutputFormat {
      FullLine,           // Полная строка
      LineWithNumber,     // N:content
      FileLineWithNumber, // file:N:content
      FileLine,           // file:content
      Fragment,           // Фрагмент с указателем
  }
  ```

- [ ] Цветной вывод (`--color`):
  - `auto` — если stdout — терминал
  - `always` — всегда
  - `never` — никогда
  - Использование `termcolor`

- [ ] Подсветка:
  - Целевые строки — зелёным (или `>`)
  - Совпадения regex — инверсным цветом
  - Указатель `^` — цветным

### Критерии завершения

- `sel -l 10-20 file.txt` без номеров
- `sel --color=always -e ERROR log.txt` с подсветкой

---

## Этап 8: Обработка ошибок и граничные случаи

**Цель**: Надёжная обработка всех ошибок.

### Задачи

- [ ] Типы ошибок:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum SelError {
      #[error("File not found: {0}")]
      FileNotFound(PathBuf),

      #[error("Invalid selector: {0}")]
      InvalidSelector(String),

      #[error("Mixed positional and non-positional selectors")]
      MixedSelectors,

      #[error("Char context requires positional selector or -e")]
      CharContextWithoutPosition,

      #[error("Invalid regex: {0}")]
      InvalidRegex(String),

      #[error("IO error: {0}")]
      Io(#[from] io::Error),
  }
  ```

- [ ] Обработка граничных случаев:
  - Пустой файл
  - Несуществующий файл
  - Неверный формат селектора
  - Отрицательные/нулевые значения N
  - Выход за границы строки/файла

- [ ] Коды возврата:
  - 0 — успех
  - 1 — ошибка (файл не найден, неверный селектор, etc.)
  - 0 но без вывода — если ничего не найдено (как `grep`)

### Критерии завершения

- Все ошибки обрабатываются с понятными сообщениями
- Коды возврата соответствуют ожиданиям

---

## Этап 9: Тестирование

**Цель**: Комплексное покрытие тестами.

### Задачи

- [ ] Unit тесты для каждого модуля:
  - `selector.rs` — все форматы
  - `reader.rs` — потоковое чтение
  - `output.rs` — форматирование

- [ ] Интеграционные тесты:
  ```
  tests/
  ├── basic.rs
  ├── selectors.rs
  ├── positions.rs
  ├── context.rs
  ├── regex.rs
  ├── multi_file.rs
  └── errors.rs
  ```

- [ ] Property-based тесты:
  - Округление фрагментов всегда валидно
  - Контекстные интервалы корректны

- [ ] Тесты с большими файлами:
  - Проверка потребления памяти
  - Производительность

### Критерии завершения

- Покрытие > 80%
- Прохождение всех тестов
- Нет memory leaks

---

## Этап 10: Оптимизация и полировка

**Цель**: Финальная оптимизация и подготовка к релизу.

### Задачи

- [ ] Профилирование:
  - `cargo flamegraph` для поиска hotspots
  - Оптимизация критических путей

- [ ] Benchmark:
  - Сравнение с альтернативами (`sed`, `grep`)
  - Большие файлы (>1GB)

- [ ] Уменьшение размера бинарника:
  - `strip = true`
  - `lto = true`
  - `panic = "abort"`

- [ ] Документация:
  - `README.md` с примерами
  - `man` page (опционально)
  - Комментарии в коде

- [ ] CI/CD:
  - GitHub Actions
  - Тесты на Linux/macOS/Windows
  - `cargo clippy` и `cargo fmt --check`

### Критерии завершения

- Бинарник < 1MB (после strip)
- Производительность comparable с `sed`
- Готовность к публикации

---

## Дополнительные идеи (будущие версии)

- Поддержка `--chars` для счёта колонок в символах Unicode
- Чтение из stdin
- Интерактивный режим
- Поддержка других форматов (JSON, CSV)
- Вывод в формате diff

---

## Порядок реализации (рекомендуется)

1. Этапы 1-2: Структура и парсинг
2. Этапы 3-4: Базовый функционал
3. Этап 5: Контекст
4. Этап 6: Regex
5. Этапы 7-10: Полировка

Ориентировочное время: 2-3 недели активной разработки.
