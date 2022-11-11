// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#446

// Converts a date and time stored within ZIP headers into a `chrono` structure.
pub fn zip_date_to_chrono(date: u16, time: u16) -> DateTime<Utc> {
    let years = (((date & 0xFE00) >> 9) + 1980).into();
    let months = ((date & 0x1E0) >> 5).into();
    let days = (date & 0x1F).into();

    let hours = ((time & 0xF800) >> 11).into();
    let mins = ((time & 0x7E0) >> 5).into();
    let secs = ((time & 0x1F) << 1).into();

    match Utc.ymd_opt(years, months, days) {
        chrono::LocalResult::Single(t) => match t.and_hms_opt(hours, mins, secs) {
            Some(dt) => dt,
            None => chrono::DateTime::<chrono::Utc>::MIN_UTC,
        },
        _ => chrono::DateTime::<chrono::Utc>::MIN_UTC,
    }
}

// Converts a `chrono` structure into a date and time stored in ZIP headers.
pub fn chrono_to_zip_time(dt: &DateTime<Utc>) -> (u16, u16) {
    let year: u16 = (((dt.date().year() - 1980) << 9) & 0xFE00).try_into().unwrap();
    let month: u16 = ((dt.date().month() << 5) & 0x1E0).try_into().unwrap();
    let day: u16 = (dt.date().day() & 0x1F).try_into().unwrap();

    let hour: u16 = ((dt.time().hour() << 11) & 0xF800).try_into().unwrap();
    let min: u16 = ((dt.time().minute() << 5) & 0x7E0).try_into().unwrap();
    let second: u16 = ((dt.time().second() >> 1) & 0x1F).try_into().unwrap();

    (hour | min | second, year | month | day)
}
