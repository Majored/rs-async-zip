// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[cfg(feature = "chrono")]
use chrono::{TimeZone, Utc};

#[test]
#[cfg(feature = "chrono")]
fn date_conversion_test() {
    let original_dt = Utc.timestamp_opt(1666544102, 0).unwrap();
    let zip_dt = crate::ZipDateTime::from_chrono(&original_dt);
    let result_dt = zip_dt.as_chrono().single().expect("expected single unique result");
    assert_eq!(result_dt, original_dt);
}
