#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use chrono::{Datelike, Timelike};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl Default for tm {
    fn default() -> Self {
        tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone: std::ptr::null(),
        }
    }
}

impl TryFrom<tm> for chrono::NaiveDateTime {
    type Error = ();

    fn try_from(val: tm) -> Result<Self, Self::Error> {
        let date = chrono::NaiveDate::from_ymd_opt(
            val.tm_year as i32 + 1900,
            (val.tm_mon + 1) as u32,
            val.tm_mday as u32,
        )
        .ok_or(())?;
        let time = chrono::NaiveTime::from_hms_opt(
            val.tm_hour as u32,
            val.tm_min as u32,
            val.tm_sec as u32,
        )
        .ok_or(())?;
        Ok(chrono::NaiveDateTime::new(date, time))
    }
}

impl TryFrom<&chrono::NaiveDateTime> for tm {
    type Error = ();

    fn try_from(val: &chrono::NaiveDateTime) -> Result<Self, Self::Error> {
        Ok(tm {
            tm_sec: val.second() as i32,
            tm_min: val.minute() as i32,
            tm_hour: val.hour() as i32,
            tm_mday: val.day() as i32,
            tm_mon: val.month0() as i32,
            tm_year: val.year() - 1900,
            ..tm::default()
        })
    }
}
