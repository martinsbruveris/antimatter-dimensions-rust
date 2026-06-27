use super::*;

#[test]
fn commas_group_thousands() {
    assert_eq!(format_with_commas("100000"), "100,000");
    assert_eq!(format_with_commas("1234567"), "1,234,567");
    assert_eq!(format_with_commas("999"), "999");
    assert_eq!(format_with_commas("1000"), "1,000");
    assert_eq!(format_with_commas("-12345"), "-12,345");
}
