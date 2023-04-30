// Copyright (c) 2021-2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[cfg(feature = "chrono")]
use chrono::{DateTime, Datelike, LocalResult, TimeZone, Timelike, Utc};

// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#446
// https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-dosdatetimetovarianttime

/// A date and time stored as per the MS-DOS representation used by ZIP files.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ZipDateTime {
    pub(crate) date: u16,
    pub(crate) time: u16,
}

impl ZipDateTime {
    /// Returns the year of this date & time.
    pub fn year(&self) -> i32 {
        (((self.date & 0xFE00) >> 9) + 1980).into()
    }

    /// Returns the month of this date & time.
    pub fn month(&self) -> u32 {
        ((self.date & 0x1E0) >> 5).into()
    }

    /// Returns the day of this date & time.
    pub fn day(&self) -> u32 {
        (self.date & 0x1F).into()
    }

    /// Returns the hour of this date & time.
    pub fn hour(&self) -> u32 {
        ((self.time & 0xF800) >> 11).into()
    }

    /// Returns the minute of this date & time.
    pub fn minute(&self) -> u32 {
        ((self.time & 0x7E0) >> 5).into()
    }

    /// Returns the second of this date & time.
    ///
    /// Note that MS-DOS has a maximum granularity of two seconds.
    pub fn second(&self) -> u32 {
        ((self.time & 0x1F) << 1).into()
    }

    /// Constructs chrono's [`DateTime`] representation of this date & time.
    ///
    /// Note that this requires the `chrono` feature.
    #[cfg(feature = "chrono")]
    pub fn as_chrono(&self) -> LocalResult<DateTime<Utc>> {
        Utc.with_ymd_and_hms(self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second())
    }

    /// Constructs this date & time from chrono's [`DateTime`] representation.
    ///
    /// Note that this requires the `chrono` feature.
    #[cfg(feature = "chrono")]
    pub fn from_chrono(dt: &DateTime<Utc>) -> Self {
        let year: u16 = (((dt.date_naive().year() - 1980) << 9) & 0xFE00).try_into().unwrap();
        let month: u16 = ((dt.date_naive().month() << 5) & 0x1E0).try_into().unwrap();
        let day: u16 = (dt.date_naive().day() & 0x1F).try_into().unwrap();

        let hour: u16 = ((dt.time().hour() << 11) & 0xF800).try_into().unwrap();
        let min: u16 = ((dt.time().minute() << 5) & 0x7E0).try_into().unwrap();
        let second: u16 = ((dt.time().second() >> 1) & 0x1F).try_into().unwrap();

        ZipDateTime { date: year | month | day, time: hour | min | second }
    }
}
