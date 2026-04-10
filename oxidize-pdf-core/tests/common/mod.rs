/// Count the number of /Type /Page entries in PDF bytes (excluding /Type /Pages).
pub fn count_pages(bytes: &[u8]) -> usize {
    let content = String::from_utf8_lossy(bytes);
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            (trimmed.contains("/Type /Page") || trimmed.contains("/Type/Page"))
                && !trimmed.contains("/Pages")
        })
        .count()
}
