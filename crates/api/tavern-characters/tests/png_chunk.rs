//! PNG tEXt chunk 读写。私有 base64 通过公开读写接口覆盖。

use tavern_characters::png::{PngError, minimal_png, read_chara, write_chara};

#[test]
fn write_then_read_chara() {
    let png = write_chara(&minimal_png(), br#"{"name":"A"}"#).unwrap();
    assert_eq!(read_chara(&png).unwrap(), br#"{"name":"A"}"#);
}

#[test]
fn payloads_of_every_length_roundtrip() {
    for n in 0..40usize {
        let payload: Vec<u8> = (0..n).map(|i| b'a' + (i % 26) as u8).collect();
        let png = write_chara(&minimal_png(), &payload).unwrap();
        assert_eq!(read_chara(&png).unwrap(), payload, "len {n}");
    }
}

#[test]
fn rewrite_replaces_not_duplicates() {
    let once = write_chara(&minimal_png(), br#"{"name":"first"}"#).unwrap();
    let twice = write_chara(&once, br#"{"name":"second"}"#).unwrap();
    assert_eq!(read_chara(&twice).unwrap(), br#"{"name":"second"}"#);
    // 重写不应让文件随次数线性变大
    let thrice = write_chara(&twice, br#"{"name":"second"}"#).unwrap();
    assert_eq!(thrice.len(), twice.len());
}

#[test]
fn rejects_non_png_and_missing_chunk() {
    assert!(matches!(read_chara(b"nope"), Err(PngError::NotPng)));
    assert!(matches!(
        read_chara(&minimal_png()),
        Err(PngError::NoCharaChunk)
    ));
}
