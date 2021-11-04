use qr_code::QrCode;
use std::io::Cursor;

/// Converts `input` in base64 and returns a data url
pub fn to_data_url<T: AsRef<[u8]>>(input: T, content_type: &str) -> String {
    let base64 = base64::encode(input.as_ref());
    format!("data:{};base64,{}", content_type, base64)
}

/// Creates QR containing `message` and encode it in data url
pub(crate) fn create_bmp_base64_qr(message: &str) -> crate::error::Result<String> {
    let qr = QrCode::new(message.as_bytes())?;

    let bmp = qr.to_bmp().add_white_border(4)?.mul(4)?;

    let mut cursor = Cursor::new(vec![]);
    bmp.write(&mut cursor)?;
    Ok(to_data_url(cursor.into_inner(), "image/bmp"))
}
