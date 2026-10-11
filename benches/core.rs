use kedr_core::{
    HistoryLimit, TextBuffer,
    document::Document,
    project::{Project, ScanOptions},
};
use std::{fs, hint::black_box, path::PathBuf, time::Instant};
const WARMUP: usize = 10;
const SAMPLES: usize = 40;

fn measure(name: &str, engine: &str, bytes: usize, operations: usize, mut batch: impl FnMut()) {
    for _ in 0..WARMUP {
        batch();
    }
    for sample in 0..SAMPLES {
        let start = Instant::now();
        batch();
        let ns = start.elapsed().as_nanos();
        println!("{name},{engine},{bytes},{sample},{operations},{ns}");
    }
}
fn measure_fresh<T>(
    name: &str,
    engine: &str,
    bytes: usize,
    operations: usize,
    mut setup: impl FnMut() -> T,
    mut batch: impl FnMut(&mut T),
) {
    for _ in 0..WARMUP {
        let mut value = setup();
        batch(&mut value);
    }
    for sample in 0..SAMPLES {
        let mut value = setup();
        let start = Instant::now();
        batch(&mut value);
        let ns = start.elapsed().as_nanos();
        println!("{name},{engine},{bytes},{sample},{operations},{ns}");
    }
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut n = 0;
        loop {
            let p = std::env::temp_dir().join(format!("kedr-bench-{}-{n}", std::process::id()));
            match fs::create_dir(&p) {
                Ok(()) => return Self(p),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
                Err(e) => panic!("Benchmark fixture: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() {
    println!("workload,engine,text_bytes,sample,batch_operations,elapsed_ns");
    for bytes in [1024, 64 * 1024, 1024 * 1024, 8 * 1024 * 1024] {
        let at = bytes / 2;
        let ops = 128;
        let mut buffer = TextBuffer::with_history("x".repeat(bytes), HistoryLimit::Disabled);
        measure("local_insert_delete", "piece_table", bytes, ops, || {
            for _ in 0..ops {
                buffer.replace(black_box(at)..at, black_box("a")).unwrap();
                buffer.replace(at..at + 1, "").unwrap();
                black_box(buffer.len_bytes());
            }
        });
        assert!(buffer.equals(&"x".repeat(bytes)));
        let mut text = "x".repeat(bytes);
        text.reserve(1);
        measure("local_insert_delete", "std_string", bytes, ops, || {
            for _ in 0..ops {
                text.insert_str(black_box(at), black_box("a"));
                text.replace_range(at..at + 1, "");
                black_box(text.len());
            }
        });
        assert_eq!(text, "x".repeat(bytes));
        let positions: Vec<_> = (0..ops)
            .map(|i| ((i as u64 * 2654435761) % bytes as u64) as usize)
            .collect();
        let mut buffer = TextBuffer::with_history("x".repeat(bytes), HistoryLimit::Disabled);
        measure("scattered_insert_delete", "piece_table", bytes, ops, || {
            for &at in &positions {
                buffer.replace(black_box(at)..at, "a").unwrap();
                buffer.replace(at..at + 1, "").unwrap();
                black_box(buffer.len_bytes());
            }
        });
        let mut text = "x".repeat(bytes);
        text.reserve(1);
        measure("scattered_insert_delete", "std_string", bytes, ops, || {
            for &at in &positions {
                text.insert(black_box(at), 'a');
                text.replace_range(at..at + 1, "");
                black_box(text.len());
            }
        });
    }
    let source = "x".repeat(1024 * 1024);
    let operations = 1024;
    measure_fresh(
        "fragmented_insert",
        "piece_table",
        source.len(),
        operations,
        || TextBuffer::with_history(source.clone(), HistoryLimit::Disabled),
        |b| {
            for i in 0..operations {
                let at = (i * 2654435761usize) % b.len_bytes();
                b.replace(black_box(at)..at, "a").unwrap();
            }
            black_box(b.piece_count());
        },
    );
    measure_fresh(
        "fragmented_insert",
        "std_string",
        source.len(),
        operations,
        || {
            let mut s = source.clone();
            s.reserve(operations);
            s
        },
        |s| {
            for i in 0..operations {
                let at = (i * 2654435761usize) % s.len();
                s.insert(black_box(at), 'a');
            }
            black_box(s.len());
        },
    );
    let bytes = 1024 * 1024;
    let mut buffer = TextBuffer::new("x".repeat(bytes));
    buffer.replace(bytes / 2..bytes / 2, "a").unwrap();
    measure("undo_redo", "piece_table", bytes, 128, || {
        for _ in 0..128 {
            assert!(buffer.undo());
            assert!(buffer.redo());
            black_box(buffer.len_bytes());
        }
    });
    measure("materialize", "piece_table", bytes, 32, || {
        for _ in 0..32 {
            black_box(buffer.to_text());
        }
    });
    let content = "0123456789\n".repeat(bytes / 11);
    let buffer = TextBuffer::new(content.clone());
    measure(
        "cursor_lookup_near_end",
        "piece_table",
        content.len(),
        8,
        || {
            for _ in 0..8 {
                black_box(
                    buffer
                        .byte_to_position(black_box(content.len() - 1))
                        .unwrap(),
                );
            }
        },
    );
    let fixture = Fixture::new();
    let file = fixture.0.join("document.txt");
    fs::write(&file, "x".repeat(bytes)).unwrap();
    measure("open_warm_file", "document", bytes, 4, || {
        for _ in 0..4 {
            black_box(Document::open(&file).unwrap());
        }
    });
    let mut document = Document::open(&file).unwrap();
    let mut toggle = false;
    measure("save_staged_synced", "document", bytes, 1, || {
        toggle = !toggle;
        // Includes the small edit as well as save; this is an edit-and-save workload.
        document
            .buffer
            .replace(0..1, if toggle { "a" } else { "b" })
            .unwrap();
        document.save().unwrap();
    });
    let project = fixture.0.join("project");
    fs::create_dir(&project).unwrap();
    for d in 0..10 {
        let dir = project.join(format!("d{d}"));
        fs::create_dir(&dir).unwrap();
        for f in 0..100 {
            fs::write(dir.join(format!("f{f}.py")), "pass\n").unwrap();
        }
    }
    measure("scan_1000_files", "project", 0, 1, || {
        let scan = Project::scan(&project, &ScanOptions::default()).unwrap();
        assert_eq!(scan.files.len(), 1000);
        black_box(scan);
    });
}
