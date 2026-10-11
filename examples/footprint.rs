use kedr_core::{HistoryLimit, TextBuffer};
use std::hint::black_box;
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "lines".into());
    let text = "0123456789\n".repeat(4 * 1024 * 1024 / 11);
    let mut buffer = TextBuffer::with_history(text, HistoryLimit::Disabled);
    match mode.as_str() {
        "lines" => {
            black_box(buffer.byte_to_position(buffer.len_bytes() - 1).unwrap());
        }
        "fragmented" => {
            for i in 0..4096 {
                let at = (i * 2654435761usize) % buffer.len_bytes();
                buffer.replace(at..at, "a").unwrap();
            }
        }
        "history" => {
            buffer = TextBuffer::new(buffer.to_text());
            for i in 0..1024 {
                let at = (i * 2654435761usize) % buffer.len_bytes();
                buffer.replace(at..at, "a").unwrap();
            }
        }
        "clones" => {
            let copies: Vec<_> = (0..32).map(|_| buffer.clone()).collect();
            black_box(copies);
        }
        _ => panic!("unknown case"),
    }
    black_box(buffer);
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(line) = status.lines().find(|l| l.starts_with("VmHWM:"))
    {
        println!("{}", line.split_whitespace().nth(1).unwrap());
    }
}
