use kedr_core::{Edit, EditError, HistoryLimit, TextBuffer};

#[test]
fn insert_replace_delete_across_pieces() {
    let mut b = TextBuffer::new("abcdef");
    b.replace(3..3, "XYZ").unwrap();
    assert_eq!(b.to_text(), "abcXYZdef");
    b.replace(1..8, "!").unwrap();
    assert_eq!(b.to_text(), "a!f");
    b.replace(0..3, "").unwrap();
    assert!(b.is_empty());
    assert_eq!(b.line_count(), 1);
    b.replace(0..0, "🌲\n").unwrap();
    assert_eq!(b.line_count(), 2);
}
#[test]
fn unicode_boundaries_reject_without_mutation() {
    let mut b = TextBuffer::new("a🌲б");
    let revision = b.revision();
    assert_eq!(b.replace(2..3, "x"), Err(EditError::NotCharacterBoundary));
    assert_eq!(b.replace(0..99, ""), Err(EditError::InvalidRange));
    assert_eq!(b.revision(), revision);
    assert_eq!(b.to_text(), "a🌲б");
    b.replace(1..5, "木").unwrap();
    assert_eq!(b.to_text(), "a木б");
}
#[test]
fn undo_redo_and_redo_invalidation() {
    let mut b = TextBuffer::new("a");
    b.replace(1..1, "б").unwrap();
    assert!(b.undo());
    assert_eq!(b.to_text(), "a");
    assert!(b.redo());
    assert_eq!(b.to_text(), "aб");
    b.undo();
    b.replace(0..1, "c").unwrap();
    assert!(!b.redo());
}
#[test]
fn no_op_keeps_revision_and_history() {
    let mut b = TextBuffer::new("hello");
    b.replace(0..5, "hello").unwrap();
    assert_eq!(b.revision(), 0);
    assert!(!b.undo());
}
#[test]
fn history_is_bounded_by_delta_bytes() {
    let mut b = TextBuffer::with_history("", HistoryLimit::Bytes(4));
    for _ in 0..10 {
        b.replace(b.len_bytes()..b.len_bytes(), "a").unwrap();
    }
    assert!(b.history_bytes() <= 4);
    let mut undos = 0;
    while b.undo() {
        undos += 1;
    }
    assert_eq!(undos, 4);
    assert_eq!(b.to_text(), "aaaaaa");
}
#[test]
fn oversized_change_drops_invalidated_history() {
    let mut b = TextBuffer::with_history("a", HistoryLimit::Bytes(4));
    b.replace(0..1, "b").unwrap();
    b.replace(0..1, "12345678").unwrap();
    assert_eq!(b.history_bytes(), 0);
    assert!(!b.undo());
}
#[test]
fn disabled_history_has_no_undo() {
    let mut b = TextBuffer::with_history("x", HistoryLimit::Disabled);
    b.replace(0..1, "y").unwrap();
    assert!(!b.undo());
    assert_eq!(b.history_bytes(), 0);
}
#[test]
fn compaction_preserves_text_and_history() {
    let mut b = TextBuffer::new("abcdef");
    b.replace(2..4, "木\n").unwrap();
    let text = b.to_text();
    b.compact();
    assert_eq!(b.to_text(), text);
    assert_eq!(b.piece_count(), 1);
    b.undo();
    assert_eq!(b.to_text(), "abcdef");
    b.redo();
    assert_eq!(b.to_text(), text);
}
#[test]
fn line_positions_use_unicode_scalar_columns() {
    let mut b = TextBuffer::new("a🌲\nб\r\n");
    b.replace(0..0, "木").unwrap();
    assert_eq!(b.line_count(), 3);
    for at in b
        .to_text()
        .char_indices()
        .map(|(at, _)| at)
        .chain([b.len_bytes()])
    {
        let (line, col) = b.byte_to_position(at).unwrap();
        assert_eq!(b.position_to_byte(line, col).unwrap(), at);
    }
    assert_eq!(b.position_to_byte(99, 0), Err(EditError::InvalidPosition));
    assert_eq!(b.byte_to_position(1), Err(EditError::NotCharacterBoundary));
}
#[test]
fn search_matches_across_piece_boundaries() {
    let mut b = TextBuffer::new("hello world");
    b.replace(5..5, "🌲").unwrap();
    assert_eq!(b.find_all("o🌲 w"), vec![4..11]);
    assert!(b.find_all("").is_empty());
}
#[test]
fn batch_is_one_undo_and_original_coordinates() {
    let mut b = TextBuffer::new("abc def ghi");
    b.apply_edits(&[
        Edit {
            range: 8..11,
            replacement: "🌲".into(),
        },
        Edit {
            range: 0..3,
            replacement: "longer".into(),
        },
    ])
    .unwrap();
    assert_eq!(b.to_text(), "longer def 🌲");
    b.undo();
    assert_eq!(b.to_text(), "abc def ghi");
    assert!(!b.undo());
}
#[test]
fn invalid_batch_never_partially_applies() {
    let mut b = TextBuffer::new("abc");
    let edits = [
        Edit {
            range: 0..1,
            replacement: "X".into(),
        },
        Edit {
            range: 1..99,
            replacement: "Y".into(),
        },
    ];
    assert!(b.apply_edits(&edits).is_err());
    assert_eq!(b.to_text(), "abc");
    let edits = [
        Edit {
            range: 0..2,
            replacement: "X".into(),
        },
        Edit {
            range: 1..3,
            replacement: "Y".into(),
        },
    ];
    assert_eq!(b.apply_edits(&edits), Err(EditError::OverlappingEdits));
    assert_eq!(b.revision(), 0);
}
#[test]
fn random_edits_match_string_reference_and_undo_all() {
    for seed in 1..=8 {
        let mut state = seed as u64;
        let mut b = TextBuffer::new("a🌲\nб");
        let mut expected = b.to_text();
        let mut history = Vec::new();
        for _ in 0..1000 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let boundaries: Vec<_> = expected
                .char_indices()
                .map(|(at, _)| at)
                .chain([expected.len()])
                .collect();
            let i = state as usize % boundaries.len();
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = state as usize % boundaries.len();
            let range = boundaries[i.min(j)]..boundaries[i.max(j)];
            let replacement = ["", "x", "🌲", "б\n", "木e\u{301}"][state as usize % 5];
            let before = expected.clone();
            expected.replace_range(range.clone(), replacement);
            if before != expected {
                history.push(before);
            }
            b.replace(range, replacement).unwrap();
            assert_eq!(b.to_text(), expected);
            assert_eq!(
                b.line_count(),
                expected.bytes().filter(|c| *c == b'\n').count() + 1
            );
            if state.is_multiple_of(17) {
                b.compact();
            }
        }
        for before in history.into_iter().rev() {
            assert!(b.undo());
            assert_eq!(b.to_text(), before);
        }
        assert!(!b.undo());
    }
}
