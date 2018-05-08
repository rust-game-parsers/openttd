use chrono::prelude::*;
use std;
use std::ffi::CString;

named!(pub read_string<&[u8], CString>, do_parse!(
    s: map_res!(take_till!(|v| v == 0), |arr| CString::new(arr)) >>
    take!(1) >>
    (s)
));

pub fn datetime_from_ts<T: Into<i64>>(ts: T) -> DateTime<Utc> {
    DateTime::from_utc(NaiveDateTime::from_timestamp(ts.into(), 0), Utc)
}

pub trait ByteWriter {
    /// Encode self and write bytes into buffer
    fn write_pkt(&self, out: &mut Vec<u8>) -> std::io::Result<()>;
}
