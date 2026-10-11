/// Units used by a zero-based line column. Byte offsets remain UTF-8.
/// UTF-16 is suitable for protocol adapters; neither mode counts grapheme clusters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionEncoding {
    #[default]
    UnicodeScalar,
    Utf16,
}
