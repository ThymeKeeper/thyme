// Test file for raw string parsing
fn main() {
    let s1 = r"simple raw string";
    let s2 = r#"raw string with quotes "inside""#;
    let s3 = r##"raw string with # and quotes "#"##;
    let s4 = r"";  // Empty raw string
    let s5 = r#""#; // Empty raw string with delimiter
}
