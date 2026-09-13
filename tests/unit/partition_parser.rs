use std::fs::File;
use std::io::Read;

#[test]
fn test_mbr_fixture_file_exists_and_signature() {
    let mut file =
        File::open("tests/fixtures/mbr_partition_table.bin").expect("MBR fixture must exist");
    let mut buf = [0u8; 512];
    file.read_exact(&mut buf).expect("read MBR fixture");

    // Check 0x55AA signature at 510-511
    assert_eq!(buf[510], 0x55);
    assert_eq!(buf[511], 0xaa);
}

#[test]
fn test_gpt_fixture_file_exists_and_header() {
    let mut file =
        File::open("tests/fixtures/gpt_partition_table.bin").expect("GPT fixture must exist");
    let mut buf = vec![0u8; 17408];
    file.read_exact(&mut buf).expect("read GPT fixture");

    // LBA 0 has 0x55AA
    assert_eq!(buf[510], 0x55);
    assert_eq!(buf[511], 0xaa);

    // LBA 1 at offset 512 has "EFI PART"
    assert_eq!(&buf[512..520], b"EFI PART");
}
