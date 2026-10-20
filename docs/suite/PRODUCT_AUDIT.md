# Полный продуктовый аудит Kedr: без скидки за красивые логотипы

## Вердикт диванного эксперта

> «Вы принесли каталог из 33 названий, а запускаться умеет один редактор. Это не suite: это работающий фундамент плюс 32 обещания. Зелёный CI доказывает, что написанные проверки прошли, а не что ненаписанные продукты внезапно появились».

Это критика состояния продукта, не авторов. Ниже отдельно помечены воспроизведённые баги, отсутствующие функции и пробелы доказательств. Ограничение квоты само по себе не баг. Честно указанная planned-функция — не обман, но и не поставленная возможность.

**Всего 90 пунктов:** 58 общих/CodeCedar + 32 отдельных незавершённых продукта. Список не является доказательством отсутствия любых других ошибок. Исходная ревизия: `ed088a67342034a76da59a6e0c4ae034b388ec6a`.

## Что реально проверено

- Каталог: все 33 записи отдельно, guides, SVG, явные implementation/null, реальные manifest/package/main.rs для preview. Статический PASS — не запуск приложения.
- Единственный runtime: CodeCedar (`kedr-studio`), core/workbench/UI/app tests, живые Rust definition/references, build/Clippy, installer Python tests и native capture generation. Числа и финальные результаты — в разделе приёмки ниже.
- Остальные 32: `NOT_IMPLEMENTED`, поэтому runtime/install/feature tests **не выполнялись: исполняемого продукта нет**. CodeCedar не тестировался вместо PyCedar/IdeaCedar и т. д.
- Предыдущий hosted Studio run [39305573342](https://github.com/drainedgodw/kedr/actions/runs/39305573342) на базовой ревизии: все четыре jobs success — Linux/Windows/macOS desktop и Arch package. Включая Windows installer. Это не hosted acceptance новых исправлений.
- Четыре regression tests сначала упали на исходном коде, затем прошли после изменений. Отдельно проверены parent-path rebinding, file permissions, сохранение новых/посторонних journal entries и отказ от symlinked session.
- Первичный live-прогон без установленного rust-analyzer завершился ошибкой framing; после установки реального компонента прогон повторён. Не скрываем первичный отказ и не приписываем его изменениям recovery.
- На физическом Windows/Mac/Wayland, в screen reader и при аппаратном отключении питания в этой сессии проверок нет. Нет независимого penetration test, supply-chain certification или сравнительного JetBrains performance benchmark.

## Матрица всех продуктов

| Продукт | Статус исполнения | Где проверять / почему запуск недоступен |
| --- | --- | --- |
| [CodeCedar](../../products/codecedar.md) | RUNNABLE PREVIEW — проверен общий workspace, не полная IDE | `apps/studio`, единый настоящий binary |
| [IdeaCedar](../../products/ideacedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | JVM project import and build models; Semantic completion, navigation and refactoring; JVM debugging, tests and framework tooling |
| [PyCedar](../../products/pycedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Interpreter, virtual environment and packaging management; Python analysis, refactoring and tests; Debugging, notebooks and web framework workflows |
| [WebCedar](../../products/webcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | JS/TS semantic services and frameworks; Node/package manager/test runner integration; Browser and Node debugging |
| [PhpCedar](../../products/phpcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | PHP/Composer project model; Semantic tooling and framework integration; PHP tests and debugging |
| [RubyCedar](../../products/rubycedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Ruby environments and dependency management; Ruby/Rails semantic tooling; Tests and debugger integration |
| [GoCedar](../../products/gocedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Go modules and workspace model; Semantic tooling and refactoring; Tests, debugging and profiling integration |
| [CppCedar](../../products/cppcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | CMake/toolchain and compilation database support; C/C++ semantic services and refactoring; Native debugging, sanitizers and test runners |
| [RustCedar](../../products/rustcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Cargo/rust-analyzer project integration; Versioned diagnostics, completion and refactoring; Debugging, tests and profiling integration |
| [DotCedar](../../products/dotcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Solution/MSBuild/NuGet project model; C#/F# semantic services and refactoring; .NET debugging and game engine workflows |
| [DataCedar](../../products/datacedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Permission-aware database drivers and connection management; SQL completion and schema navigation; Query execution, result editing and safe transactions |
| [MetaCedar](../../products/metacedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Language definition and projectional editing; Type systems and generation; Language testing and migration tooling |
| [BoxCedar](../../products/boxcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Verified packages and channel management; Installation, removal and rollback; Local project launch and version management |
| [SharpCedar](../../products/sharpcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Permission-scoped Visual Studio integration; C# inspections and refactoring; Solution navigation and test workflows |
| [SharpCppCedar](../../products/sharpcppcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Visual Studio C++ project integration; C++ analysis and refactoring; Native tests and navigation |
| [PeekCedar](../../products/peekcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Assembly metadata browsing; Legal/permission-aware decompilation workflows; Source/symbol navigation |
| [TraceCedar](../../products/tracecedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Runtime sampling and tracing; Timeline and call tree exploration; Reproducible profile comparison |
| [MemoryCedar](../../products/memorycedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Managed runtime heap snapshots; Allocation/retention analysis; Leak investigation and profile comparison |
| [CoverCedar](../../products/covercedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Runtime coverage collection; Test runner integration; Branch/statement coverage and comparison |
| [TrackCedar](../../products/trackcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Issues, permissions and workflow engine; Boards, search and reporting; Audit trails and import/export |
| [BuildCedar](../../products/buildcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Build agents, queues and isolation; Pipeline configuration and secrets boundaries; Artifacts, logs and build history |
| [HubCedar](../../products/hubcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Identity and authentication boundaries; Role/permission administration; Audited service integrations |
| [LabCedar](../../products/labcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Reproducible notebook execution; Data access and compute isolation; Collaboration and shareable results |
| [QualityCedar](../../products/qualitycedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Static analysis pipelines; Baselines, policy gates and reports; CI integration and findings navigation |
| [PairCedar](../../products/paircedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Explicitly authorized shared sessions; Editor synchronization and permission controls; Connection recovery and collaboration security |
| [GateCedar](../../products/gatecedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Secure remote host connections; Workspace discovery and lifecycle; Version/protocol compatibility checks |
| [CloudCedar](../../products/cloudcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Isolated environment provisioning; Resource/policy controls; Workspace lifecycle and auditability |
| [LearnCedar](../../products/learncedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Courses and exercise workflows; Local test-based feedback; Progress and curriculum authoring |
| [AgentCedar](../../products/agentcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Explicit capabilities and execution permissions; Reviewable plans, patches and test evidence; Provider abstraction and cancellation |
| [AICedar](../../products/aicedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Local/remote provider choice; Explicit data-sharing and context limits; Consent-based assistance without cloud dependency |
| [AirCedar](../../products/aircedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Audited agent sessions and workspace isolation; Provider/protocol interoperability; Review, cancellation and governance |
| [MarketCedar](../../products/marketcedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Signed manifests and permission declarations; Compatibility/dependency validation; Review, updates and revocation |
| [MatterCedar](../../products/mattercedar.md) | NOT IMPLEMENTED — runtime-тест недоступен | Research the product scope before implementation; Document data boundaries and service contracts; Approve a testable specification before feature claims |

## Правила приоритета и статуса

P0 — риск потери/раскрытия данных или зависания критического пути. P1 — необходимые ежедневные workflows/надёжность. P2 — качество взаимодействия/поставка. P3 — отдельные будущие продукты: нельзя исправить их отсутствие ещё одной иконкой.

`FIXED` — исправление текущего этапа с указанным regression/contract test; `FIXED_GATE` — добавлена проверка, не реализован весь feature family; `OPEN` — не исправлено; `NOT_IMPLEMENTED` — runtime отсутствует. Процент готовности из количества закрытых строк не вычисляется.

## Единый список и критерии закрытия

### K-001 · P0 · FIXED — Временный save-файл был доступнее приватного исходника

**Доказательство:** `src/document.rs; staging_is_private_before_any_document_bytes_are_written`.
**Критерий закрытия:** Создавать Unix stage сразу с 0600; подтвердить до записи байтов. Windows ACL — отдельная проверка.

### K-002 · P0 · FIXED — Save мог создать файл, который сам Open откажется читать

**Доказательство:** `tests/files.rs; oversized_edited_document_cannot_be_saved_into_an_unopenable_file`.
**Критерий закрытия:** Лимит 16 MiB проверяется до файловых операций; диск и dirty-буфер сохраняются при отказе.

### K-003 · P0 · FIXED — Подмена открытого пути симлинком не останавливала сохранение

**Доказательство:** `tests/files.rs; save_rejects_rebound_symlink_target_without_replacing_it; save_rejects_rebound_parent_directory`.
**Критерий закрытия:** Отказывать при наблюдаемой смене канонического пути/необычном типе; не заменять ссылку. Не выдавать это за TOCTOU-защиту.

### K-004 · P0 · FIXED — Удалённый recovery-снимок мог воскреснуть из старой ревизии

**Доказательство:** `apps/studio/src/recovery.rs; deleting_latest_snapshot_does_not_resurrect_retired_revisions`.
**Критерий закрытия:** Удалять старые номерные ревизии до выбранной; новые/посторонние/tmp не трогать; при ошибке оставлять выбранную.

### K-005 · P1 · FIXED — Каталог проверял логотипы лучше, чем наличие продукта

**Доказательство:** `scripts/audit_products.py; scripts/tests/test_product_audit.py`.
**Критерий закрытия:** Явное implementation/null для всех 33; package/manifest/main.rs, уникальность runtime, отрицательные тесты и артефакт CI. Это не runtime-тест.

### K-006 · P2 · FIXED — Руководство одновременно обещало recovery и отрицало его наличие

**Доказательство:** `products/codecedar.md, раздел Limits`.
**Критерий закрытия:** Убрать противоречие и назвать границы: plaintext checkpoint, не autosave проекта.

### K-007 · P0 · OPEN — Сохранение всё ещё способно заморозить интерфейс

**Доказательство:** `src/document.rs; crates/kedr-workbench/src/workspace.rs; docs/studio/FILES.md`.
**Критерий закрытия:** Worker save с immutable snapshot, версиями и отказом от устаревшего завершения; замедленный filesystem не блокирует UI.

### K-008 · P0 · OPEN — Save all — последовательный цикл, не транзакция

**Доказательство:** `crates/kedr-workbench/src/workspace.rs::save_all`.
**Критерий закрытия:** Предварительная проверка всех файлов, явный отчёт частичного успеха и recovery; не обещать атомарность нескольких файлов.

### K-009 · P1 · OPEN — Нет Save As, особенно болезненно для пропавших файлов

**Доказательство:** `docs/studio/RECOVERY.md; docs/suite/AUDIT.md`.
**Критерий закрытия:** Явный безопасный выбор нового пути, overwrite-confirmation, обновление identity и тест отмены.

### K-010 · P1 · OPEN — Конфликт на диске заканчивается отказом, а не инструментом разрешения

**Доказательство:** `crates/kedr-ui/src/shell/disk.rs; docs/studio/FILES.md`.
**Критерий закрытия:** Diff base/disk/buffer, выбор пользователя, merge/сохранение копии без потери правок.

### K-011 · P0 · OPEN — Нет блокировки против параллельного внешнего писателя

**Доказательство:** `src/document.rs::save; docs/studio/RECOVERY.md`.
**Критерий закрытия:** Документированный concurrency contract и race/fault tests; проверка перед rename не равна writer lock.

### K-012 · P0 · OPEN — Полная power-loss durability обычного Save не доказана

**Доказательство:** `src/document.rs::save; после rename нет fsync родительского каталога`.
**Критерий закрытия:** Тестируемый commit protocol и fault injection, включая sync каталога, ошибки после commit и разные ОС/ФС.

### K-013 · P1 · OPEN — Материализация recovery ещё выполняется на render thread

**Доказательство:** `apps/studio/src/main.rs; crates/kedr-workbench/src/recovery.rs::capture`.
**Критерий закрытия:** Снять immutable snapshot быстро; сборку строк/сравнение перенести с кадра, замерить большие dirty-tabs.

### K-014 · P0 · OPEN — Последние правки до checkpoint можно потерять

**Доказательство:** `apps/studio/src/main.rs; docs/studio/RECOVERY.md`.
**Критерий закрытия:** Задать измеримый RPO, испытать kill до/во время/после checkpoint; не обещать per-keystroke persistence.

### K-015 · P1 · OPEN — Recovery сохраняет секреты обычным текстом

**Доказательство:** `docs/studio/RECOVERY.md`.
**Критерий закрытия:** Понятные privacy-controls, исключения, опциональное защищённое хранение; threat model до слова «secure».

### K-016 · P1 · OPEN — Старые backup живут бессрочно без retention policy

**Доказательство:** `docs/studio/RECOVERY.md`.
**Критерий закрытия:** Настраиваемый срок/объём, подтверждаемый purge, видимое потребление диска и тесты нескольких сессий.

### K-017 · P1 · OPEN — Повреждённый newest journal оставляется без мастера ремонта

**Доказательство:** `apps/studio/src/recovery.rs::discover`.
**Критерий закрытия:** Экспорт/диагностика/явное удаление повреждённого снимка; не молча откатываться к устаревшему тексту.

### K-018 · P2 · OPEN — Recovery не восстанавливает undo, selections и всю сессию

**Доказательство:** `crates/kedr-workbench/src/recovery.rs::Draft; docs/studio/RECOVERY.md`.
**Критерий закрытия:** Версионированный session contract с миграцией/квотами и безопасным отказом от повреждённых данных.

### K-019 · P1 · OPEN — Квоты discovery могут скрыть часть сохранённых сессий

**Доказательство:** `apps/studio/src/recovery.rs::discover`.
**Критерий закрытия:** Пагинация/управление retained data; тест более 32 offers, 1024 entries, 128 revisions и суммарных 64 MiB.

### K-020 · P0 · OPEN — Зависшее OS I/O способно задержать recovery и выход

**Доказательство:** `apps/studio/src/main.rs; docs/studio/RECOVERY.md`.
**Критерий закрытия:** Явная UX-политика долгого shutdown, отмена где доступна, тест зависшего I/O; не блокировать render loop.

### K-021 · P1 · OPEN — Preferences используют фиксированный PID temp и синхронную запись

**Доказательство:** `apps/studio/src/preferences.rs::write`.
**Критерий закрытия:** Уникальный stage, очистка при всех ошибках, fault tests и неблокирующее сохранение настроек.

### K-022 · P1 · OPEN — Поиск по проекту не видит несохранённый код

**Доказательство:** `crates/kedr-workbench/src/search.rs`.
**Критерий закрытия:** Версионированные overlay открытых буферов; дедупликация с диском и stale-result tests.

### K-023 · P2 · OPEN — Поиск только literal/case-sensitive, без привычных режимов

**Доказательство:** `crates/kedr-workbench/src/search.rs::search_project`.
**Критерий закрытия:** Case/word/regex/file-mask режимы с ограничением стоимости и тестами Unicode/пустых совпадений.

### K-024 · P1 · OPEN — Индекс файлов устаревает после внешнего создания и удаления

**Доказательство:** `crates/kedr-workbench/src/workspace.rs; docs/studio/FILES.md`.
**Критерий закрытия:** Фоновый rescan/watch с поколениями; не терять dirty-tabs при смене tree.

### K-025 · P2 · OPEN — Project scan использует фиксированные исключения вместо полноценного ignore

**Доказательство:** `src/project.rs::Project::scan`.
**Критерий закрытия:** Определённая политика .gitignore/локальных исключений, вложенные правила, UI включения игнорируемого.

### K-026 · P1 · OPEN — Нельзя нормально создавать, переименовывать и удалять файлы из проекта

**Доказательство:** `docs/suite/AUDIT.md; crates/kedr-workbench/src/workspace.rs`.
**Критерий закрытия:** Безопасные file operations с подтверждением dirty/conflict, обновлением индекса и rollback/recovery.

### K-027 · P1 · OPEN — Каждый semantic jump запускает новый rust-analyzer

**Доказательство:** `crates/kedr-workbench/src/lsp/process.rs; docs/studio/LANGUAGE.md`.
**Критерий закрытия:** Persistent session, didChange/didClose, workspace generation, restart/cancel и stale guards.

### K-028 · P1 · OPEN — Нет semantic completion — IDE без автодополнения пока редактор

**Доказательство:** `docs/studio/LANGUAGE.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Completion с версией документа, cancellation, корректными UTF-16 edits и undo.

### K-029 · P1 · OPEN — Нет hover/signature help

**Доказательство:** `docs/studio/LANGUAGE.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Ненавязчивый async hover/signature с sanitize содержимого и lifecycle tests.

### K-030 · P1 · OPEN — Нет живой LSP diagnostics; Cargo report относится к saved sources

**Доказательство:** `docs/studio/DIAGNOSTICS.md; docs/studio/LANGUAGE.md`.
**Критерий закрытия:** publishDiagnostics по версии/URI, очистка старых сообщений, overlays и устойчивость к restart.

### K-031 · P1 · OPEN — Нет semantic rename; textual replacement не считается

**Доказательство:** `docs/suite/AUDIT.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Preview multi-file WorkspaceEdit, версии, конфликтные решения и восстановление при частичном отказе.

### K-032 · P1 · OPEN — Нет safe code actions и formatter workflow

**Доказательство:** `docs/suite/PARITY.md; docs/studio/LANGUAGE.md`.
**Критерий закрытия:** Подтверждаемые edits, диапазоны, undo, dirty/version и отказ от server commands без отдельного согласия.

### K-033 · P1 · OPEN — Навигация не покрывает стандартную библиотеку и внешние зависимости

**Доказательство:** `docs/studio/LANGUAGE.md, scanned workspace targets`.
**Критерий закрытия:** Явная модель read-only внешних sources и URI/trust boundaries, не просто снять starts_with guard.

### K-034 · P2 · OPEN — При нескольких definitions выбирается первая

**Доказательство:** `docs/studio/LANGUAGE.md`.
**Критерий закрытия:** Chooser с preview и клавиатурой, stale snapshot guard; не угадывать нужную цель.

### K-035 · P2 · OPEN — Ошибки запуска analyzer не всегда объясняют причину

**Доказательство:** `crates/kedr-workbench/src/lsp/process.rs, stderr discarded; первичный локальный live-run завершился Invalid or oversized LSP header до установки компонента`.
**Критерий закрытия:** Bounded диагностический stderr/exit status и понятная подсказка; после установки сервера живой тест проверять отдельно.

### K-036 · P1 · OPEN — Stop убивает прямого потомка, не всё дерево процессов

**Доказательство:** `crates/kedr-workbench/src/tasks.rs; docs/studio/LANGUAGE.md`.
**Критерий закрытия:** Unix process groups/Windows Job Objects или эквивалент; тесты детей/внуков и закрытия окна.

### K-037 · P1 · OPEN — Фаза завершения Cargo может опередить дренирование stdout/stderr

**Доказательство:** `crates/kedr-workbench/src/tasks.rs::pipe и try_wait`.
**Критерий закрытия:** Финализировать после bounded drain, учитывать унаследованные pipes; тест больших хвостов/UTF-8/ошибок.

### K-038 · P1 · OPEN — Build/test интеграция ограничена Cargo

**Доказательство:** `crates/kedr-workbench/src/tasks.rs::TaskKind`.
**Критерий закрытия:** Реальные adapters toolchain/project models, окружения и явное разрешение выполнения для других языков.

### K-039 · P2 · OPEN — Cargo test output не является test tree и coverage UI

**Доказательство:** `crates/kedr-workbench/src/tasks.rs; docs/suite/PARITY.md`.
**Критерий закрытия:** Структурированные результаты, rerun отдельного теста, failures, lifecycle; coverage отдельно.

### K-040 · P2 · OPEN — Нет полноценной модели run configurations

**Доказательство:** `docs/suite/AUDIT.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Версионированные args/env/cwd, секреты не в открытом JSON, проверка запуска/отмены.

### K-041 · P1 · OPEN — Нет Git status/diff/history/commit workflow

**Доказательство:** `docs/suite/PARITY.md`.
**Критерий закрытия:** Сначала безопасный read-only status/diff; изменяющие операции с подтверждением и dirty guards.

### K-042 · P1 · OPEN — Output console не заменяет терминал

**Доказательство:** `docs/suite/PARITY.md; crates/kedr-workbench/src/tasks.rs`.
**Критерий закрытия:** Реальный PTY, resize, input, Unicode, shell exit, process-tree lifecycle на целевых ОС.

### K-043 · P1 · OPEN — Нет отладчика

**Доказательство:** `docs/suite/PARITY.md`.
**Критерий закрытия:** DAP/native adapter, breakpoints, stepping, threads/frames/variables и fault tests.

### K-044 · P2 · OPEN — Нет multi-cursor/column selection workflow

**Доказательство:** `docs/studio/PRODUCT.md; редактор хранит один cursor/anchor`.
**Критерий закрытия:** Набор selection ranges, атомарное undo и тесты перекрытий/Unicode, а не декоративная кнопка.

### K-045 · P2 · OPEN — Курсор движется по scalars, не grapheme clusters

**Доказательство:** `products/codecedar.md; crates/kedr-workbench/src/editor.rs`.
**Критерий закрытия:** Определить grapheme/bidi policy; combining marks, emoji ZWJ, flags, selection и backspace tests.

### K-046 · P2 · OPEN — Syntax highlighting — line lexer, не parser/type analysis

**Доказательство:** `products/codecedar.md`.
**Критерий закрытия:** Корректность multiline constructs, incremental parsing и замеры; не называть цвета semantic analysis.

### K-047 · P2 · OPEN — Длинная строка отображается окном 8 KiB, нет wrap/minimap

**Доказательство:** `products/codecedar.md`.
**Критерий закрытия:** Navigation/find/caret/scroll acceptance на pathological lines; не путать render limit с потерей текста.

### K-048 · P2 · OPEN — Жёсткие квоты ограничивают реальные большие проекты

**Доказательство:** `src/project.rs; products/codecedar.md; docs/studio/LANGUAGE.md`.
**Критерий закрытия:** Понятные лимиты/частичность в UI, стресс-тесты; 16 MiB/32 tabs/5000 files не универсальная IDE.

### K-049 · P1 · OPEN — Custom editor не предоставляет полный screen-reader text model

**Доказательство:** `products/codecedar.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Text/selection semantics и ручные NVDA/VoiceOver/Orca сценарии; labels недостаточно.

### K-050 · P1 · OPEN — Физические IME/Wayland/high-DPI сценарии ещё не приняты

**Доказательство:** `docs/studio/VALIDATION.md`.
**Критерий закрытия:** Реальные target-machine тесты composition, clipboard, focus, portal, scaling и fonts.

### K-051 · P2 · OPEN — Keymap — single-stroke subset, не полная совместимость

**Доказательство:** `docs/suite/SHORTCUTS.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Chord/double-modifier/context dispatch с конфликтами и физическими клавиатурами.

### K-052 · P2 · OPEN — Нет завершённой локализации и покрытия всех письменностей

**Доказательство:** `products/codecedar.md; docs/suite/PARITY.md`.
**Критерий закрытия:** Externalized strings, plural/forms/layout expansion, fallback fonts и RTL acceptance.

### K-053 · P1 · OPEN — Распространение — unsigned preview artifacts, не trusted update channel

**Доказательство:** `products/codecedar.md; docs/suite/INSTALL.md`.
**Критерий закрытия:** Подпись/проверка происхождения, rollback, обновление, retention релизов и reproducible-build policy.

### K-054 · P2 · OPEN — macOS tarball не равен подписанному .app и notarized installer

**Доказательство:** `products/codecedar.md; .github/workflows/studio.yml`.
**Критерий закрытия:** Настоящий bundle, icons/open associations, подпись/notarization и target-machine upgrade/uninstall.

### K-055 · P1 · FIXED_GATE — Заявленный MSRV всего desktop workspace не проверялся отдельным CI gate

**Доказательство:** `Cargo.toml rust-version; .github/workflows/core.yml проверял только default root; добавлен studio msrv-workspace`.
**Критерий закрытия:** cargo +1.88.0 check --locked --workspace --all-targets локально; отдельный hosted job, результат нового run отдельно.

### K-056 · P1 · OPEN — Нет актуального полного UI latency/memory/endurance baseline

**Доказательство:** `PERFORMANCE.md; docs/studio/VALIDATION.md`.
**Критерий закрытия:** p50/p95/p99 edit/open/search/save, RSS и долгий сеанс на документированном железе; core bench не UI bench.

### K-057 · P1 · OPEN — Journal child-kill tests не доказывают полный GUI crash/restore

**Доказательство:** `docs/studio/RECOVERY.md; apps/studio/src/recovery.rs tests`.
**Критерий закрытия:** GUI edit → checkpoint → kill → restart → restore плюс disk-full/permission/interrupted-commit и upgrade tests.

### K-058 · P2 · OPEN — Генерация screenshots не равна визуальной и accessibility приёмке

**Доказательство:** `scripts/ui_capture.py; docs/studio/VALIDATION.md`.
**Критерий закрытия:** Индивидуальный review сцен, отдельные many-offers/nonmatching-workspace сценарии и физические платформы.

### P-ideacedar · P3 · NOT_IMPLEMENTED — IdeaCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/ideacedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: JVM project import and build models; Semantic completion, navigation and refactoring; JVM debugging, tests and framework tooling. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-pycedar · P3 · NOT_IMPLEMENTED — PyCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/pycedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Interpreter, virtual environment and packaging management; Python analysis, refactoring and tests; Debugging, notebooks and web framework workflows. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-webcedar · P3 · NOT_IMPLEMENTED — WebCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/webcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: JS/TS semantic services and frameworks; Node/package manager/test runner integration; Browser and Node debugging. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-phpcedar · P3 · NOT_IMPLEMENTED — PhpCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/phpcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: PHP/Composer project model; Semantic tooling and framework integration; PHP tests and debugging. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-rubycedar · P3 · NOT_IMPLEMENTED — RubyCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/rubycedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Ruby environments and dependency management; Ruby/Rails semantic tooling; Tests and debugger integration. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-gocedar · P3 · NOT_IMPLEMENTED — GoCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/gocedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Go modules and workspace model; Semantic tooling and refactoring; Tests, debugging and profiling integration. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-cppcedar · P3 · NOT_IMPLEMENTED — CppCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/cppcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: CMake/toolchain and compilation database support; C/C++ semantic services and refactoring; Native debugging, sanitizers and test runners. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-rustcedar · P3 · NOT_IMPLEMENTED — RustCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/rustcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Cargo/rust-analyzer project integration; Versioned diagnostics, completion and refactoring; Debugging, tests and profiling integration. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-dotcedar · P3 · NOT_IMPLEMENTED — DotCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/dotcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Solution/MSBuild/NuGet project model; C#/F# semantic services and refactoring; .NET debugging and game engine workflows. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-datacedar · P3 · NOT_IMPLEMENTED — DataCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/datacedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Permission-aware database drivers and connection management; SQL completion and schema navigation; Query execution, result editing and safe transactions. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-metacedar · P3 · NOT_IMPLEMENTED — MetaCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/metacedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Language definition and projectional editing; Type systems and generation; Language testing and migration tooling. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-boxcedar · P3 · NOT_IMPLEMENTED — BoxCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/boxcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Verified packages and channel management; Installation, removal and rollback; Local project launch and version management. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-sharpcedar · P3 · NOT_IMPLEMENTED — SharpCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/sharpcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Permission-scoped Visual Studio integration; C# inspections and refactoring; Solution navigation and test workflows. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-sharpcppcedar · P3 · NOT_IMPLEMENTED — SharpCppCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/sharpcppcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Visual Studio C++ project integration; C++ analysis and refactoring; Native tests and navigation. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-peekcedar · P3 · NOT_IMPLEMENTED — PeekCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/peekcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Assembly metadata browsing; Legal/permission-aware decompilation workflows; Source/symbol navigation. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-tracecedar · P3 · NOT_IMPLEMENTED — TraceCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/tracecedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Runtime sampling and tracing; Timeline and call tree exploration; Reproducible profile comparison. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-memorycedar · P3 · NOT_IMPLEMENTED — MemoryCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/memorycedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Managed runtime heap snapshots; Allocation/retention analysis; Leak investigation and profile comparison. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-covercedar · P3 · NOT_IMPLEMENTED — CoverCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/covercedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Runtime coverage collection; Test runner integration; Branch/statement coverage and comparison. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-trackcedar · P3 · NOT_IMPLEMENTED — TrackCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/trackcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Issues, permissions and workflow engine; Boards, search and reporting; Audit trails and import/export. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-buildcedar · P3 · NOT_IMPLEMENTED — BuildCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/buildcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Build agents, queues and isolation; Pipeline configuration and secrets boundaries; Artifacts, logs and build history. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-hubcedar · P3 · NOT_IMPLEMENTED — HubCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/hubcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Identity and authentication boundaries; Role/permission administration; Audited service integrations. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-labcedar · P3 · NOT_IMPLEMENTED — LabCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/labcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Reproducible notebook execution; Data access and compute isolation; Collaboration and shareable results. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-qualitycedar · P3 · NOT_IMPLEMENTED — QualityCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/qualitycedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Static analysis pipelines; Baselines, policy gates and reports; CI integration and findings navigation. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-paircedar · P3 · NOT_IMPLEMENTED — PairCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/paircedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Explicitly authorized shared sessions; Editor synchronization and permission controls; Connection recovery and collaboration security. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-gatecedar · P3 · NOT_IMPLEMENTED — GateCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/gatecedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Secure remote host connections; Workspace discovery and lifecycle; Version/protocol compatibility checks. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-cloudcedar · P3 · NOT_IMPLEMENTED — CloudCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/cloudcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Isolated environment provisioning; Resource/policy controls; Workspace lifecycle and auditability. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-learncedar · P3 · NOT_IMPLEMENTED — LearnCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/learncedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Courses and exercise workflows; Local test-based feedback; Progress and curriculum authoring. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-agentcedar · P3 · NOT_IMPLEMENTED — AgentCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/agentcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Explicit capabilities and execution permissions; Reviewable plans, patches and test evidence; Provider abstraction and cancellation. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-aicedar · P3 · NOT_IMPLEMENTED — AICedar: нет самостоятельного приложения/backend

**Доказательство:** `products/aicedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Local/remote provider choice; Explicit data-sharing and context limits; Consent-based assistance without cloud dependency. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-aircedar · P3 · NOT_IMPLEMENTED — AirCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/aircedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Audited agent sessions and workspace isolation; Provider/protocol interoperability; Review, cancellation and governance. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-marketcedar · P3 · NOT_IMPLEMENTED — MarketCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/marketcedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Signed manifests and permission declarations; Compatibility/dependency validation; Review, updates and revocation. Нужны реальный host/package, integration/security tests и проверяемая поставка.

### P-mattercedar · P3 · NOT_IMPLEMENTED — MatterCedar: нет самостоятельного приложения/backend

**Доказательство:** `products/mattercedar.md; products/catalog.json implementation=null`.
**Критерий закрытия:** Реализовать и проверить: Research the product scope before implementation; Document data boundaries and service contracts; Approve a testable specification before feature claims. Нужны реальный host/package, integration/security tests и проверяемая поставка.

## Что уже исправлено, а что дальше

В этом этапе исправлены четыре воспроизведённых дефекта защиты данных, добавлен runtime-evidence inventory с отрицательными тестами, устранено противоречие в руководстве и добавлен workspace MSRV CI gate. Это не «исправлены все 90 пунктов за один коммит».

Следующая последовательность: (1) asynchronous save с version-safe completion и conflict UX; (2) recovery privacy/retention и fault injection; (3) persistent Rust LSP, затем completion/hover/live diagnostics; (4) file operations и Git read-only; (5) PTY/DAP; (6) реальные platform/accessibility acceptance. Отдельные продукты требуют backend/spec/security — общий shell их не заменяет.

## Воспроизводимые команды

```bash
cargo test --locked --workspace
cargo test --locked -p kedr-workbench real_rust_analyzer_ -- --ignored --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked -p kedr-studio
cargo +1.88.0 check --locked --workspace --all-targets
python3 -m unittest discover -s packaging/tests -v
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_catalog.py
python3 scripts/audit_products.py --out product-audit.json
LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1600x1000x24" python3 scripts/ui_capture.py --binary target/debug/kedr-studio
```

Python inventory требует Python 3.11+ (`tomllib`). Live tests требуют реально установленный rust-analyzer; GUI capture — Xvfb/Mesa и графические библиотеки. Default tests содержат два ignored live tests, запускаемые отдельно. Не принимать «Ran 0 tests» за приёмку.

Машиночитаемый список: [product-audit.json](product-audit.json). Политика общей готовности: [PARITY.md](PARITY.md). Предшествующий аудит: [AUDIT.md](AUDIT.md).

## Приёмка текущих исправлений (локальная)

- **191** default Rust tests passed; **2 live rust-analyzer** tests passed отдельно.
- **19** Linux installer Python tests и **14** product-contract tests passed.
- Все **32 planned products** отдельно отклонены настоящим CLI installer без изменения файлов.
- Настоящий собранный binary: install → verify → reinstall/upgrade → verify → `--help` → uninstall в изолированном HOME с пробелами; hash установленного payload совпал.
- `fmt`, workspace/all-targets Clippy с `-D warnings`, native debug build, catalog/keymap validation и `diff --check` passed.
- **Rust 1.88.0**, `cargo check --locked --workspace --all-targets`: passed локально.
- Core release tests passed; Unicode endurance: **10000** проверенных операций и **3** сверенных background snapshots. Это не UI latency benchmark.
- **51** native scene с проверенными PNG/binary SHA-256; отдельно просмотрены **4**: compact recovery, delete confirmation, 32-offer compact stress, nonmatching-workspace light. Controls/text/Later/Cancel помещаются; остальные 47 сцен в этом этапе не проходили повторный визуальный review.
- Binary SHA-256: `96d75dd0f7470ae53d552bcfa916d768f8c09ec73429bece5d55a0b8a7a442b8`.
- Первые попытки live/native проверки потребовали установки rust-analyzer/libXcursor. В установочном harness исправлен неверно ожидаемый путь binary. Финальные прогоны после исправления окружения/harness успешны; первоначальные неудачи не спрятаны и не засчитаны как passed.
- Новый hosted CI после публикации проверяется отдельно. Ни локальные тесты, ни предыдущий зелёный run не означают автоматически зелёный новый run.

## Subsequent distribution revision — original audit is not a release certificate

The next revision adds deb/rpm dependency-managed packages, clean-image native
install acceptance, Windows static CRT/PE dependency auditing and macOS app/DMG
building. See [distribution contracts](DISTRIBUTION.md) and
[actual revision evidence](DISTRIBUTION_VALIDATION.md). K-053/K-054 remain open:
unsigned previews and ad-hoc signing are not signed release/update/notarization.
The original audit workflow 39387663786 completed with Linux/Arch/MSRV success but
Windows/macOS failure in recovery deletion. Discovery path canonicalization is
now fixed locally; new hosted platform results must be checked independently.
The 32 missing product implementations remain missing.
