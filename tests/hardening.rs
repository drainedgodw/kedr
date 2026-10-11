use kedr_core::{Edit, EditBatch, EditError, PositionEncoding, TextBuffer, Version};

#[test]
fn utf16_surrogate_halves_are_rejected() {
    let b = TextBuffer::new("a🌲b\n");
    assert_eq!(b.len_utf16(), 5);
    assert_eq!(
        b.position_to_byte_in(0, 1, PositionEncoding::Utf16)
            .unwrap(),
        1
    );
    assert_eq!(
        b.position_to_byte_in(0, 2, PositionEncoding::Utf16),
        Err(EditError::InvalidPosition)
    );
    assert_eq!(
        b.position_to_byte_in(0, 3, PositionEncoding::Utf16)
            .unwrap(),
        5
    );
    assert_eq!(
        b.byte_to_position_in(5, PositionEncoding::Utf16).unwrap(),
        (0, 3)
    );
    assert_eq!(b.byte_to_position(5).unwrap(), (0, 2));
}
#[test]
fn utf16_roundtrips_after_fragmentation_and_compaction() {
    let mut b = TextBuffer::new(format!(
        "{}\r\n{}\n",
        "🌲e\u{301}б".repeat(600),
        "木🙂".repeat(600)
    ));
    for _ in 0..50 {
        b.replace(4..4, "λ🙂").unwrap();
    }
    for compact in [false, true] {
        if compact {
            b.compact();
        }
        let text = b.to_text();
        let mut line = 0;
        let mut column = 0;
        assert_eq!(b.len_utf16(), text.encode_utf16().count());
        for (at, ch) in text.char_indices() {
            assert_eq!(
                b.byte_to_position_in(at, PositionEncoding::Utf16).unwrap(),
                (line, column)
            );
            assert_eq!(
                b.position_to_byte_in(line, column, PositionEncoding::Utf16)
                    .unwrap(),
                at
            );
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += ch.len_utf16();
            }
        }
        assert_eq!(
            b.byte_to_position_in(text.len(), PositionEncoding::Utf16)
                .unwrap(),
            (line, column)
        );
    }
}
#[test]
fn snapshot_utf16_index_stays_immutable() {
    let mut b = TextBuffer::new("🌲\n");
    let s = b.snapshot();
    b.replace(0..4, "木").unwrap();
    assert_eq!(s.len_utf16(), 3);
    assert_eq!(b.len_utf16(), 2);
    assert_eq!(
        s.position_to_byte_in(0, 2, PositionEncoding::Utf16)
            .unwrap(),
        4
    );
    assert_eq!(
        s.position_to_byte_in(0, 1, PositionEncoding::Utf16),
        Err(EditError::InvalidPosition)
    );
}
#[test]
fn invalidation_log_is_bounded_and_gap_is_explicit() {
    let mut b = TextBuffer::new("a");
    let first = b.version();
    b.set_change_log_capacity(2);
    for i in 0..4 {
        b.replace(0..1, if i % 2 == 0 { "b" } else { "a" }).unwrap();
    }
    assert_eq!(
        b.changes_since(first).err(),
        Some(EditError::ChangeLogGap {
            requested: 0,
            oldest_available: 2
        })
    );
    let retained = Version {
        document_id: b.version().document_id,
        revision: 2,
    };
    assert_eq!(
        b.changes_since(retained)
            .unwrap()
            .map(|c| c.new_revision)
            .collect::<Vec<_>>(),
        vec![3, 4]
    );
    b.take_change();
    assert_eq!(b.changes_since(retained).unwrap().count(), 2);
    b.compact();
    assert_eq!(b.revision(), 4);
    assert_eq!(b.changes_since(b.version()).unwrap().count(), 0);
}
#[test]
fn invalidation_log_detects_wrong_and_future_versions() {
    let mut b = TextBuffer::new("x");
    b.set_change_log_capacity(4);
    let other = TextBuffer::new("x");
    assert!(matches!(
        b.changes_since(other.version()),
        Err(EditError::WrongDocument { .. })
    ));
    let future = Version {
        document_id: b.version().document_id,
        revision: 9,
    };
    assert_eq!(
        b.changes_since(future).err(),
        Some(EditError::FutureRevision {
            requested: 9,
            current: 0
        })
    );
}
#[test]
fn invalidation_log_accounts_for_allocations_and_releases_them() {
    let mut b = TextBuffer::new("a");
    b.set_change_log_capacity(4);
    b.replace(0..1, "b").unwrap();
    let stats = b.memory_stats();
    assert!(stats.change_log_bytes > 0);
    assert_eq!(
        stats.reachable_bytes(),
        stats.text_bytes
            + stats.index_bytes
            + stats.node_bytes
            + stats.chunk_header_bytes
            + stats.history_container_bytes
            + stats.change_log_bytes
    );
    b.set_change_log_capacity(0);
    assert_eq!(b.memory_stats().change_log_bytes, 0);
    let old = Version {
        document_id: b.version().document_id,
        revision: 0,
    };
    assert!(matches!(
        b.changes_since(old),
        Err(EditError::ChangeLogGap { .. })
    ));
}
#[test]
fn undo_redo_and_batch_have_one_event_per_revision() {
    let mut b = TextBuffer::new("abc def");
    b.set_change_log_capacity(8);
    let original = b.version();
    let batch = EditBatch::new(
        &b.snapshot(),
        vec![
            Edit {
                range: 0..3,
                replacement: "🌲".into(),
            },
            Edit {
                range: 4..7,
                replacement: "x\ny".into(),
            },
        ],
    );
    b.apply_batch(&batch).unwrap();
    assert!(b.try_undo().unwrap());
    assert!(b.try_redo().unwrap());
    let changes: Vec<_> = b.changes_since(original).unwrap().collect();
    assert_eq!(changes.len(), 3);
    for (i, c) in changes.iter().enumerate() {
        assert_eq!(c.old_revision, i as u64);
        assert_eq!(c.new_revision, i as u64 + 1);
    }
    assert_eq!(b.to_text(), "🌲 x\ny");
}
#[test]
fn forks_keep_journal_settings_but_not_another_documents_events() {
    let mut a = TextBuffer::new("a");
    a.set_change_log_capacity(4);
    a.replace(0..1, "b").unwrap();
    let mut b = a.clone();
    let first = b.version();
    assert_eq!(b.changes_since(first).unwrap().count(), 0);
    b.replace(0..1, "c").unwrap();
    assert_eq!(b.changes_since(first).unwrap().count(), 1);
    assert!(matches!(
        b.changes_since(a.version()),
        Err(EditError::WrongDocument { .. })
    ));
}
#[test]
fn random_utf16_columns_match_direct_reference() {
    let mut b = TextBuffer::new("a🌲\n");
    let mut state = 0x12345678u64;
    for _ in 0..1500 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let text = b.to_text();
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain([text.len()])
            .collect();
        let at = boundaries[state as usize % boundaries.len()];
        b.replace(
            at..at,
            ["x", "🌲", "б\n", "e\u{301}", "\r\n"][state as usize % 5],
        )
        .unwrap();
        let text = b.to_text();
        let mut probe = state as usize % text.len();
        while !text.is_char_boundary(probe) {
            probe -= 1;
        }
        let prefix = &text[..probe];
        let line = prefix.bytes().filter(|v| *v == b'\n').count();
        let start = prefix.rfind('\n').map_or(0, |n| n + 1);
        let col = prefix[start..].encode_utf16().count();
        assert_eq!(
            b.byte_to_position_in(probe, PositionEncoding::Utf16)
                .unwrap(),
            (line, col)
        );
        assert_eq!(
            b.position_to_byte_in(line, col, PositionEncoding::Utf16)
                .unwrap(),
            probe
        );
    }
}

#[test]
fn prepared_compaction_does_not_change_content_version_or_events() {
    let mut b = TextBuffer::new("abc");
    b.set_change_log_capacity(8);
    b.replace(1..1, "🌲").unwrap();
    b.clear_history();
    let version = b.version();
    let text = b.to_text();
    let original = b.snapshot();
    let prepared = std::thread::spawn(move || original.compacted())
        .join()
        .unwrap();
    b.install_compacted(prepared).unwrap();
    assert_eq!(b.piece_count(), 1);
    assert_eq!(b.version(), version);
    assert_eq!(b.to_text(), text);
    assert_eq!(b.changes_since(version).unwrap().count(), 0);
}
#[test]
fn prepared_compaction_rejects_stale_and_wrong_documents() {
    let mut a = TextBuffer::new("a");
    let prepared = a.snapshot().compacted();
    a.replace(0..1, "b").unwrap();
    assert!(matches!(
        a.install_compacted(prepared),
        Err(EditError::StaleRevision { .. })
    ));
    assert_eq!(a.to_text(), "b");
    let b = TextBuffer::new("b");
    assert!(matches!(
        a.install_compacted(b.snapshot().compacted()),
        Err(EditError::WrongDocument { .. })
    ));
}
