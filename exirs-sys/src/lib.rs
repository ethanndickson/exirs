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

impl TryFrom<&EXIPDateTime> for chrono::NaiveDateTime {
    type Error = ();

    fn try_from(dt: &EXIPDateTime) -> Result<Self, Self::Error> {
        let tm = dt.dateTime;
        let date = chrono::NaiveDate::from_ymd_opt(
            tm.tm_year as i32 + 1900,
            (tm.tm_mon + 1) as u32,
            tm.tm_mday as u32,
        )
        .ok_or(())?;
        let nanosecs = if (dt.presenceMask & FRACT_PRESENCE as u8) != 0 && dt.fSecs.offset <= 8 {
            dt.fSecs.value * 10_u32.pow(8 - dt.fSecs.offset as u32)
        } else {
            0
        };
        let time = chrono::NaiveTime::from_hms_nano_opt(
            tm.tm_hour as u32,
            tm.tm_min as u32,
            tm.tm_sec as u32,
            nanosecs,
        )
        .ok_or(())?;
        Ok(chrono::NaiveDateTime::new(date, time))
    }
}

impl From<&chrono::NaiveDateTime> for EXIPDateTime {
    fn from(dt: &chrono::NaiveDateTime) -> Self {
        let tm = tm {
            tm_sec: dt.second() as i32,
            tm_min: dt.minute() as i32,
            tm_hour: dt.hour() as i32,
            tm_mday: dt.day() as i32,
            tm_mon: dt.month0() as i32,
            tm_year: dt.year() - 1900,
            ..tm::default()
        };
        EXIPDateTime {
            dateTime: tm,
            fSecs: fractionalSecs {
                offset: 8,
                value: dt.nanosecond(),
            },
            TimeZone: 0,
            presenceMask: FRACT_PRESENCE as u8,
        }
    }
}

impl From<EXIFloat> for f64 {
    fn from(float: EXIFloat) -> Self {
        f64::from_bits(float.mantissa as u64 | (float.exponent as u64) << 52)
    }
}

impl From<f64> for EXIFloat {
    fn from(float: f64) -> Self {
        let float = float.to_bits();
        let mantissa = (float & ((1 << 52) - 1)) as i64;
        let exponent = ((float >> 52) & 0x7FF) as i16;
        EXIFloat { mantissa, exponent }
    }
}

#[test]
fn exip_floats() {
    assert_eq!(f64::from(EXIFloat::from(f64::INFINITY)), f64::INFINITY);
    assert_eq!(f64::from(EXIFloat::from(f64::MAX)), f64::MAX);
    assert_eq!(f64::from(EXIFloat::from(0.3)), 0.3);
    assert_eq!(
        f64::from(EXIFloat::from(std::f64::consts::E)),
        std::f64::consts::E
    );
    assert_eq!(
        f64::from(EXIFloat::from(std::f64::consts::PI)),
        std::f64::consts::PI
    );
    assert_eq!(
        f64::from(EXIFloat::from(std::f64::consts::TAU)),
        std::f64::consts::TAU
    );
    // EXIFloats are unsigned
    assert_eq!(f64::from(EXIFloat::from(f64::NEG_INFINITY)), f64::INFINITY);
    assert_eq!(f64::from(EXIFloat::from(f64::MIN)), f64::MAX);
}

#[test]
fn time_conversion() {
    let date = chrono::NaiveDate::from_ymd_opt(2012, 7, 31).unwrap();
    let time = chrono::NaiveTime::from_hms_micro_opt(13, 33, 55, 839).unwrap();
    let dt = chrono::NaiveDateTime::new(date, time);
    assert_eq!(
        dt,
        chrono::NaiveDateTime::try_from(&EXIPDateTime::try_from(&dt).unwrap()).unwrap()
    );
    // todo: more
}
