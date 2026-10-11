//! Long sessions, individual-operation tails and retained-snapshot checks.
use kedr_core::{HistoryLimit, PositionEncoding, Snapshot, TextBuffer};
use std::{
    fs,
    hint::black_box,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args.get(1).map_or("unicode", String::as_str);
    let operations = args
        .get(2)
        .map_or(10000, |v| v.parse::<usize>().expect("operation count"));
    assert!(operations > 0 && operations <= 2_000_000);
    let out = args.get(3).map_or_else(
        || std::env::temp_dir().join("kedr-endurance"),
        PathBuf::from,
    );
    fs::create_dir_all(&out).unwrap();
    let mut expected = match case {
        "ascii" => "a".repeat(16 * 1024 * 1024),
        "unicode" => "a🌲e\u{301}б\r\n".repeat(2560),
        _ => panic!("case must be ascii or unicode"),
    };
    let initial_bytes = expected.len();
    let mut buffer =
        TextBuffer::with_history(expected.clone(), HistoryLimit::Bytes(8 * 1024 * 1024));
    buffer.set_history_step_limit(128);
    buffer.set_change_log_capacity(256);
    let (send, recv) = mpsc::sync_channel::<(Snapshot, Arc<str>)>(2);
    let reader = thread::spawn(move || {
        let mut checked = 0;
        while let Ok((snapshot, reference)) = recv.recv() {
            assert_eq!(snapshot.to_text(), reference.as_ref());
            assert_eq!(snapshot.len_utf16(), reference.encode_utf16().count());
            checked += 1;
        }
        checked
    });
    let mut retained: Vec<(Snapshot, String)> = Vec::new();
    let mut state = 0x5eeda11u64;
    let mut samples = Vec::with_capacity(operations);
    let mut memory =
        BufWriter::new(fs::File::create(out.join(format!("{case}-memory.csv"))).unwrap());
    writeln!(memory, "operation,pieces,height,reachable_bytes,rss_kib").unwrap();
    let mut compactions = 0;
    let mut checkpoints = 0;
    for i in 0..operations {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut at = state as usize % expected.len();
        while !expected.is_char_boundary(at) {
            at -= 1;
        }
        let end = at + expected[at..].chars().next().unwrap().len_utf8();
        let mut replacement = if case == "ascii" {
            if &expected[at..end] == "a" { "b" } else { "a" }
        } else {
            ["a", "🌲", "б", "\n", "λ", "木", "🙂"][state as usize % 7]
        };
        if &expected[at..end] == replacement {
            replacement = "!";
        }
        let maintain = buffer.piece_count() > 65536;
        let start = Instant::now();
        buffer.replace(at..end, replacement).unwrap();
        if maintain {
            buffer.compact();
            compactions += 1;
        }
        let ns = start.elapsed().as_nanos();
        samples.push(u64::try_from(ns).unwrap());
        expected.replace_range(at..end, replacement);
        if i.is_multiple_of(4096) {
            let snapshot = buffer.snapshot();
            let reference: Arc<str> = Arc::from(expected.clone());
            match send.try_send((snapshot, reference)) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(_)) => {}
                Err(e) => panic!("background reader stopped: {e}"),
            }
        }
        if (i + 1).is_multiple_of(65536) || i + 1 == operations {
            assert!(buffer.equals(&expected));
            assert_eq!(buffer.len_utf16(), expected.encode_utf16().count());
            let mut probe = (state >> 16) as usize % expected.len();
            while !expected.is_char_boundary(probe) {
                probe -= 1;
            }
            let prefix = &expected[..probe];
            let row = prefix.bytes().filter(|b| *b == b'\n').count();
            let begin = prefix.rfind('\n').map_or(0, |p| p + 1);
            let col = prefix[begin..].encode_utf16().count();
            assert_eq!(
                buffer
                    .byte_to_position_in(probe, PositionEncoding::Utf16)
                    .unwrap(),
                (row, col)
            );
            assert_eq!(
                buffer
                    .position_to_byte_in(row, col, PositionEncoding::Utf16)
                    .unwrap(),
                probe
            );
            retained.push((buffer.snapshot(), expected.clone()));
            if retained.len() > 2 {
                retained.remove(0);
            }
            for (snapshot, reference) in &retained {
                assert_eq!(snapshot.to_text(), *reference);
            }
            let rss = proc_kib("VmRSS:").map_or(String::new(), |v| v.to_string());
            writeln!(
                memory,
                "{},{},{},{},{}",
                i + 1,
                buffer.piece_count(),
                buffer.tree_height(),
                buffer.memory_stats().reachable_bytes(),
                rss
            )
            .unwrap();
            checkpoints += 1;
        }
        black_box(buffer.revision());
    }
    assert_eq!(buffer.revision(), operations as u64);
    assert!(buffer.equals(&expected));
    drop(send);
    let background_checks = reader.join().unwrap();
    assert!(background_checks > 0);
    let mut file = BufWriter::new(fs::File::create(out.join(format!("{case}.csv"))).unwrap());
    writeln!(file, "operation,elapsed_ns").unwrap();
    for (i, ns) in samples.iter().enumerate() {
        writeln!(file, "{i},{ns}").unwrap();
    }
    file.flush().unwrap();
    memory.flush().unwrap();
    let peak = proc_kib("VmHWM:");
    let metadata = format!(
        "{{\"case\":\"{case}\",\"operations\":{operations},\"initial_bytes\":{initial_bytes},\"final_bytes\":{},\"final_pieces\":{},\"tree_height\":{},\"compactions\":{compactions},\"reference_checkpoints\":{checkpoints},\"background_checks\":{background_checks},\"peak_rss_kib\":{},\"history_step_limit\":128,\"change_log_capacity\":256,\"timing_includes_compaction\":true}}\n",
        buffer.len_bytes(),
        buffer.piece_count(),
        buffer.tree_height(),
        peak.map_or("null".into(), |v| v.to_string())
    );
    fs::write(out.join(format!("{case}.json")), metadata).unwrap();
    println!(
        "{case}: {operations} operations checked; {compactions} compactions; {background_checks} background snapshots verified"
    );
}

fn proc_kib(field: &str) -> Option<usize> {
    fs::read_to_string("/proc/self/status").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with(field))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<usize>().ok())
    })
}
