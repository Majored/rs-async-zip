// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use chrono::{TimeZone, Utc};

#[test]
fn date_conversion_test() {
    let original_dt = Utc.timestamp(1666544102, 0);
    let (time, date) = crate::spec::date::chrono_to_zip_time(&original_dt);
    let result_dt = crate::spec::date::zip_date_to_chrono(date, time);
    assert_eq!(result_dt, original_dt);
}