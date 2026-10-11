use kedr_core::{Edit, EditBatch, EditError, HistoryLimit, TextBuffer};

#[test]
fn snapshots_survive_edits_compaction_and_owner_drop() {
    let mut b = TextBuffer::new("a🌲\nb");
    let snapshot = b.snapshot();
    b.replace(0..1, "木").unwrap();
    b.compact();
    drop(b);
    assert_eq!(snapshot.to_text(), "a🌲\nb");
    assert_eq!(snapshot.revision(), 0);
    assert_eq!(snapshot.byte_to_position(6).unwrap(), (1, 0));
}
#[test]
fn snapshots_can_be_read_on_another_thread() {
    let mut b = TextBuffer::new("hello\n🌲");
    let snapshot = b.snapshot();
    let thread = std::thread::spawn(move || {
        assert_eq!(snapshot.line_count(), 2);
        snapshot.to_text()
    });
    b.replace(0..5, "goodbye").unwrap();
    assert_eq!(thread.join().unwrap(), "hello\n🌲");
}
#[test]
fn stale_batches_never_mutate() {
    let mut b = TextBuffer::new("abc");
    let batch = EditBatch {
        base_version: b.version(),
        edits: vec![Edit {
            range: 0..1,
            replacement: "X".into(),
        }],
    };
    b.replace(2..3, "Y").unwrap();
    let history = b.history_bytes();
    assert_eq!(
        b.apply_batch(&batch),
        Err(EditError::StaleRevision {
            expected: 0,
            actual: 1
        })
    );
    assert_eq!(b.to_text(), "abY");
    assert_eq!(b.history_bytes(), history);
    assert_eq!(b.revision(), 1);
}
#[test]
fn batch_change_envelope_and_undo_are_consistent() {
    let mut b = TextBuffer::new("abc def ghi");
    let batch = EditBatch {
        base_version: b.version(),
        edits: vec![
            Edit {
                range: 0..3,
                replacement: "🌲".into(),
            },
            Edit {
                range: 8..11,
                replacement: "x\ny".into(),
            },
        ],
    };
    let change = b.apply_batch(&batch).unwrap().unwrap();
    assert_eq!(change.old_range, 0..11);
    assert_eq!(change.inserted_bytes, 12);
    assert_eq!((change.old_line_count, change.new_line_count), (1, 2));
    assert_eq!((change.old_revision, change.new_revision), (0, 1));
    assert_eq!(b.to_text(), "🌲 def x\ny");
    assert_eq!(b.take_change(), Some(change));
    assert!(b.last_change().is_none());
    b.undo();
    assert_eq!(b.to_text(), "abc def ghi");
    assert_eq!(b.revision(), 2);
}
#[test]
fn no_op_batch_does_not_create_an_event() {
    let mut b = TextBuffer::new("abc");
    let batch = EditBatch {
        base_version: b.version(),
        edits: vec![Edit {
            range: 1..2,
            replacement: "b".into(),
        }],
    };
    assert_eq!(b.apply_batch(&batch).unwrap(), None);
    assert!(b.last_change().is_none());
}
#[test]
fn unicode_checkpoint_ranks_roundtrip_on_long_lines() {
    let text = format!("{}\n{}\n", "🌲e\u{301}б".repeat(700), "木".repeat(900));
    let b = TextBuffer::new(text.clone());
    let mut line = 0;
    let mut col = 0;
    for (at, c) in text.char_indices() {
        assert_eq!(b.byte_to_position(at).unwrap(), (line, col));
        assert_eq!(b.position_to_byte(line, col).unwrap(), at);
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    assert_eq!(b.byte_to_position(text.len()).unwrap(), (line, col));
    assert_eq!(b.position_to_byte(line, col).unwrap(), text.len());
    assert_eq!(
        b.position_to_byte(0, usize::MAX),
        Err(EditError::InvalidPosition)
    );
}
#[test]
fn streaming_search_matches_reference_after_fragmentation() {
    let mut b = TextBuffer::new("ababab🌲б\n🌲бababab");
    for _ in 0..8 {
        b.replace(2..2, "🌲ab").unwrap();
    }
    let text = b.to_text();
    for needle in ["ab", "abab", "🌲ab", "б\n🌲", "🌲", "missing", ""] {
        let expected: Vec<_> = if needle.is_empty() {
            vec![]
        } else {
            text.match_indices(needle)
                .map(|(at, _)| at..at + needle.len())
                .collect()
        };
        assert_eq!(b.find_iter(needle).collect::<Vec<_>>(), expected);
        assert_eq!(b.snapshot().find_all(needle), expected);
    }
}
#[test]
fn memory_accounting_deduplicates_shared_backing() {
    let mut b = TextBuffer::with_history("x".repeat(1024 * 1024), HistoryLimit::Disabled);
    for i in 0..1000 {
        let at = (i * 2654435761usize) % b.len_bytes();
        b.replace(at..at, "a").unwrap();
    }
    let stats = b.memory_stats();
    assert_eq!(stats.text_bytes, 1024 * 1024);
    assert!(stats.node_bytes > 0);
    assert!(b.tree_height() < 32);
    b.compact();
    assert_eq!(b.piece_count(), 1);
    assert_eq!(b.memory_stats().text_bytes, 1024 * 1024 + 1000);
}
#[test]
fn empty_and_trailing_line_positions_are_valid() {
    let mut b = TextBuffer::new("");
    assert_eq!(b.byte_to_position(0).unwrap(), (0, 0));
    assert_eq!(b.position_to_byte(0, 0).unwrap(), 0);
    b.replace(0..0, "\n\n").unwrap();
    assert_eq!(b.line_count(), 3);
    for line in 0..3 {
        assert_eq!(b.position_to_byte(line, 0).unwrap(), line);
    }
    assert_eq!(b.position_to_byte(1, 1), Err(EditError::InvalidPosition));
}

#[test]
fn batches_are_bound_to_document_identity() {
    let mut a = TextBuffer::new("a");
    let mut b = a.clone();
    let batch = EditBatch::new(
        &a.snapshot(),
        vec![Edit {
            range: 0..1,
            replacement: "b".into(),
        }],
    );
    assert!(matches!(
        b.apply_batch(&batch),
        Err(EditError::WrongDocument { .. })
    ));
    assert_eq!(b.to_text(), "a");
    a.apply_batch(&batch).unwrap();
    assert_eq!(a.to_text(), "b");
}

#[test]
fn history_step_limit_and_redo_pruning_are_safe() {
    let mut b = TextBuffer::new("");
    b.set_history_step_limit(3);
    for _ in 0..10 {
        let at = b.len_bytes();
        b.replace(at..at, "a").unwrap();
    }
    for _ in 0..3 {
        assert!(b.undo());
    }
    assert!(!b.undo());
    assert_eq!(b.to_text(), "aaaaaaa");
    b.set_history_step_limit(1);
    assert!(b.redo());
    assert!(!b.redo());
    assert_eq!(b.to_text(), "aaaaaaaa");
    b.set_history_step_limit(0);
    assert!(!b.undo());
    assert_eq!(b.history_bytes(), 0);
}

#[test]
fn memory_total_includes_inline_chunk_headers() {
    let b = TextBuffer::new("🌲");
    let s = b.memory_stats();
    assert_eq!(s.text_bytes, 0);
    assert!(s.chunk_header_bytes > 0);
    assert_eq!(
        s.reachable_bytes(),
        s.text_bytes
            + s.index_bytes
            + s.node_bytes
            + s.chunk_header_bytes
            + s.history_container_bytes
            + s.change_log_bytes
    );
}
