pub fn utf8_to_utf16(str : &str) -> Vec<u16>
{
    str.encode_utf16().chain(Some(0)).collect()
}