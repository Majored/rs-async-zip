// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::date::ZipDateTime;
#[cfg(feature = "chrono")]
use chrono::{TimeZone, Utc};

#[test]
#[cfg(feature = "chrono")]
fn date_conversion_test() {
    let original_dt = Utc.timestamp(1666544102, 0);
    let zip_dt = ZipDateTime::from_chrono(&original_dt);
    let result_dt = zip_dt.as_chrono().single().expect("expected single unique result");
    assert_eq!(result_dt, original_dt);
}
