use kedr_core::{Edit, EditBatch, TextBuffer};
fn main() {
    let mut buffer = TextBuffer::new("hello 🌲\n");
    let snapshot = buffer.snapshot();
    let job = std::thread::spawn(move || {
        // A parser/indexer may read this immutable version without a writer lock.
        assert_eq!(snapshot.line_count(), 2);
        EditBatch::new(
            &snapshot,
            vec![Edit {
                range: 0..5,
                replacement: "welcome".into(),
            }],
        )
    });
    let batch = job.join().unwrap();
    let change = buffer.apply_batch(&batch).unwrap().unwrap();
    assert_eq!(change.new_revision, 1);
    assert_eq!(buffer.to_text(), "welcome 🌲\n");
}
