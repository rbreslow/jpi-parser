use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take, take_until};
use nom::character::complete::{space0, anychar};
use nom::combinator::{eof, map_res, all_consuming};
use nom::error::{ErrorKind};
use nom::sequence::{pair, delimited};
use nom::character::complete;
use std::ops::BitXor;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ConfiguredLimits {
    pub volts_hi_times_ten: u16,
    pub volts_lo_times_ten: u16,
    pub dif: u16,
    pub cht: u16,
    pub cld: u16,
    pub tit: u16,
    pub oil_hi: u16,
    pub oil_lo: u16
}

#[derive(Clone, Debug, PartialEq)]
pub enum HeaderRecord {
    U(String),
    A(ConfiguredLimits),
    F(FuelFlowLimits),
    T(Timestamp),
    C(ConfigInfo),
    D(FlightInfo),
    L(LastHeaderRecord)
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct FuelFlowLimits {
    pub empty: u16,
    pub full: u16,
    pub warning: u16,
    pub k_factor: u16,
    pub k_factor2: u16,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Timestamp {
    pub month: u16,
    pub day: u16,
    pub year: u16,
    pub hour: u16,
    pub minute: u16,
    pub unknown: u16,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ConfigInfo {
    pub model_number: u16,
    pub feature_flags_lo: u16,
    pub feature_flags_hi: u16,
    pub unknown_flags: u16,
    pub firmware_version: u16,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct FlightInfo {
    pub flight_number: u16,
    pub length: u16
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct LastHeaderRecord {
    pub unknown: u16
}

fn not_underscore(i: &str) -> nom::IResult<&str, &str> {
    is_not("_")(i)
}

fn parse_hex2(input: &str) -> IResult<&str, u8> {
    map_res(take(2usize), |s| u8::from_str_radix(s, 16))(input)
}

fn parse_short(i: &str) -> IResult<&str, u16> {
    delimited(
        space0, // possible spaces to the left
        complete::u16, // the number
        pair(space0, alt((tag(","), eof))) // possible spaces to the right followed by a comma or end of the string
    )(i)
}

pub fn tail_number_parser(i: &str) -> IResult<&str, &str> {
    not_underscore(i)
}

pub fn configured_limits_parser(i: &str) -> IResult<&str, ConfiguredLimits> {
    let (i, volts_hi_times_ten) = parse_short(i)?;
    let (i, volts_lo_times_ten) = parse_short(i)?;
    let (i, dif) = parse_short(i)?;
    let (i, cht) = parse_short(i)?;
    let (i, cld) = parse_short(i)?;
    let (i, tit) = parse_short(i)?;
    let (i, oil_hi) = parse_short(i)?;
    let (i, oil_lo) = parse_short(i)?;

    Ok((i, ConfiguredLimits {
        volts_hi_times_ten,
        volts_lo_times_ten,
        dif,
        cht,
        cld,
        tit,
        oil_hi,
        oil_lo
    }))
}

pub fn fuel_flow_parser(i: &str) -> IResult<&str, FuelFlowLimits> {
    let (i, empty) = parse_short(i)?;
    let (i, full) = parse_short(i)?;
    let (i, warning) = parse_short(i)?;
    let (i, k_factor) = parse_short(i)?;
    let (i, k_factor2) = parse_short(i)?;

    Ok((i, FuelFlowLimits {
        empty,
        full,
        warning,
        k_factor,
        k_factor2
    }))
}

pub fn timestamp_parser(i: &str) -> IResult<&str, Timestamp> {
    let (i, month) = parse_short(i)?;
    let (i, day) = parse_short(i)?;
    let (i, year) = parse_short(i)?;
    let (i, hour) = parse_short(i)?;
    let (i, minute) = parse_short(i)?;
    let (i, unknown) = parse_short(i)?;

    Ok((i, Timestamp {
        month,
        day,
        year,
        hour,
        minute,
        unknown
    }))
}

pub fn config_info_parser(i: &str) -> IResult<&str, ConfigInfo> {
    let (i, model_number) = parse_short(i)?;
    let (i, feature_flags_lo) = parse_short(i)?;
    let (i, feature_flags_hi) = parse_short(i)?;
    let (i, unknown_flags) = parse_short(i)?;
    let (i, firmware_version) = parse_short(i)?;

    Ok((i, ConfigInfo {
        model_number,
        feature_flags_lo,
        feature_flags_hi,
        unknown_flags,
        firmware_version,
    }))
}

pub fn flight_info_parser(i: &str) -> IResult<&str, FlightInfo> {
    let (i, flight_number) = parse_short(i)?;
    let (i, length) = parse_short(i)?;

    Ok((i, FlightInfo {
        flight_number,
        length
    }))
}

pub fn last_header_record_parser(i: &str) -> IResult<&str, LastHeaderRecord> {
    let (i, unknown) = parse_short(i)?;

    Ok((i, LastHeaderRecord {
        unknown,
    }))
}

pub fn header_record_parser(line: &str) -> IResult<&str, (char, &str)> {
    let (i, _) = tag("$")(line)?;
    let (i, middle) = take_until("*")(i)?;
    let (i, _) = tag("*")(i)?;
    let (rest, checksum) = parse_hex2(i)?;
    let (i, header_record_type) = anychar(middle)?;
    let (header_record, _) = tag(",")(i)?;

    let computed_checksum = middle.bytes().fold(0u8, u8::bitxor);

    if computed_checksum != checksum {
        return Err(nom::Err::Failure(nom::error::Error::new(line, ErrorKind::Verify)))
    }

    Ok((rest, (header_record_type, header_record)))
}

pub fn parse_record(i: &str) -> IResult<&str, HeaderRecord> {
    let (_, (record_type, data)) = all_consuming(header_record_parser)(i)?;

    use HeaderRecord::*;
    match record_type {
        'U' => tail_number_parser.map(|x| U(x.to_owned())).parse(data),
        'A' => configured_limits_parser.map(|x| A(x)).parse(data),
        'F' => fuel_flow_parser.map(|x| F(x)).parse(data),
        'T' => timestamp_parser.map(|x| T(x)).parse(data),
        'C' => config_info_parser.map(|x| C(x)).parse(data),
        'D' => flight_info_parser.map(|x| D(x)).parse(data),
        'L' => last_header_record_parser.map(|x| L(x)).parse(data),
        _ => Err(nom::Err::Failure(nom::error::Error::new(i, ErrorKind::NoneOf)))
    }
}
