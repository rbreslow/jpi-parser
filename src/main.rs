use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take, take_until};
use nom::character::complete::{space0, anychar};
use nom::combinator::{eof};
use nom::error::{ErrorKind};
use nom::sequence::{pair, delimited};
use nom::character::complete;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct ConfiguredLimits {
     volts_hi_times_ten: u16,
     volts_lo_times_ten: u16,
     dif: u16,
     cht: u16,
     cld: u16,
     tit: u16,
     oil_hi: u16,
     oil_lo: u16
 }

 #[derive(Clone, Debug, PartialEq)]
 enum HeaderRecord {
     U(String),
     A(ConfiguredLimits),
     F(FuelFlowLimits),
     T(Timestamp),
     C(ConfigInfo),
     D(FlightInfo),
     L(LastHeaderRecord)
 }

 #[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct FuelFlowLimits {
     empty: u16,
     full: u16,
     warning: u16,
     k_factor: u16,
     k_factor2: u16,
 }

 #[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct Timestamp {
     month: u16,
     day: u16,
     year: u16,
     hour: u16,
     minute: u16,
     unknown: u16,
 }

 #[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct ConfigInfo {
     model_number: u16,
     feature_flags_lo: u16,
     feature_flags_hi: u16,
     unknown_flags: u16,
     firmware_version: u16,
 }

 #[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct FlightInfo {
     flight_number: u16,
     length: u16
 }

 #[derive(Clone, Copy, Default, Debug, PartialEq)]
 struct LastHeaderRecord {
     unknown: u16
 }

 fn not_underscore(i: &str) -> nom::IResult<&str, &str> {
     is_not("_")(i)
 }

 fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
     u8::from_str_radix(input, 16)
 }

macro_rules! comma_delim {
    ($parser:expr) => {
        delimited(
            space0, // possible spaces to the left
            $parser, // the number
            pair(space0, alt((tag(","), eof))) // possible spaces to the right followed by a comma or end of the string
        )
    };
}

 fn tail_number_parser(i: &str) -> IResult<&str, &str> {
     not_underscore(i)
 }

 fn configured_limits_parser(i: &str) -> IResult<&str, ConfiguredLimits> {
     let mut parse = comma_delim!(complete::u16);

     let (i, volts_hi_times_ten) = parse(i)?;
     let (i, volts_lo_times_ten) = parse(i)?;
     let (i, dif) = parse(i)?;
     let (i, cht) = parse(i)?;
     let (i, cld) = parse(i)?;
     let (i, tit) = parse(i)?;
     let (i, oil_hi) = parse(i)?;
     let (i, oil_lo) = parse(i)?;

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

fn fuel_flow_parser(i: &str) -> IResult<&str, FuelFlowLimits> {
    let mut parse = comma_delim!(complete::u16);

    let (i, empty) = parse(i)?;
    let (i, full) = parse(i)?;
    let (i, warning) = parse(i)?;
    let (i, k_factor) = parse(i)?;
    let (i, k_factor2) = parse(i)?;

    Ok((i, FuelFlowLimits {
        empty,
        full,
        warning,
        k_factor,
        k_factor2
    }))
}

fn timestamp_parser(i: &str) -> IResult<&str, Timestamp> {
    let mut parse = comma_delim!(complete::u16);

    let (i, month) = parse(i)?;
    let (i, day) = parse(i)?;
    let (i, year) = parse(i)?;
    let (i, hour) = parse(i)?;
    let (i, minute) = parse(i)?;
    let (i, unknown) = parse(i)?;

    Ok((i, Timestamp {
        month,
        day,
        year,
        hour,
        minute,
        unknown
    }))
}

fn config_info_parser(i: &str) -> IResult<&str, ConfigInfo> {
    let mut parse = comma_delim!(complete::u16);

    let (i, model_number) = parse(i)?;
    let (i, feature_flags_lo) = parse(i)?;
    let (i, feature_flags_hi) = parse(i)?;
    let (i, unknown_flags) = parse(i)?;
    let (i, firmware_version) = parse(i)?;

    Ok((i, ConfigInfo {
        model_number,
        feature_flags_lo,
        feature_flags_hi,
        unknown_flags,
        firmware_version,
    }))
}

fn flight_info_parser(i: &str) -> IResult<&str, FlightInfo> {
    let mut parse = comma_delim!(complete::u16);

    let (i, flight_number) = parse(i)?;
    let (i, length) = parse(i)?;

    Ok((i, FlightInfo {
        flight_number,
        length
    }))
}

fn last_header_record_parser(i: &str) -> IResult<&str, LastHeaderRecord> {
    let mut parse = comma_delim!(complete::u16);

    let (i, unknown) = parse(i)?;

    Ok((i, LastHeaderRecord {
        unknown,
    }))
}

 fn header_record_parser(i: &str) -> IResult<&str, (char, &str)> {
     let (i, _) = tag("$")(i)?;
     let (i, header_record_type) = anychar(i)?;
     let (i, _) = tag(",")(i)?;
     let (i, header_record) = take_until("*")(i)?;
     let (i, _) = tag("*")(i)?;
     let (i, checksum) = take(2usize)(i)?;

     let mut computed_checksum: u8 = 0;
     for byte in header_record.bytes() {
         computed_checksum ^= byte;
     }

     if computed_checksum == from_hex(checksum).unwrap() {
         println!("checksum is heckin valid") // TODO: return error if checksum fails
     }

     Ok((i, (header_record_type, header_record)))
 }

fn parse_record(i: &str) -> IResult<&str, HeaderRecord> {
    let (_, (record_type, data)) = header_record_parser(i)?;

    use HeaderRecord::*;
    match record_type {
        'U' => tail_number_parser.map(|x| U(x.to_owned())).parse(data),
        'A' => configured_limits_parser.map(|x| A(x)).parse(data),
        'F' => fuel_flow_parser.map(|x| F(x)).parse(data),
        'T' => timestamp_parser.map(|x| T(x)).parse(data),
        'C' => config_info_parser.map(|x| C(x)).parse(data),
        'D' => flight_info_parser.map(|x| D(x)).parse(data),
        'L' => last_header_record_parser.map(|x| L(x)).parse(data),
        _ => Err(nom::Err::Error(nom::error::Error::new(i, ErrorKind::NoneOf)))
    }
}

 #[test]
 fn test() {
     assert_eq!(tail_number_parser("N51SW__"), Ok(("__", "N51SW")));
     assert_eq!(tail_number_parser("__N51SW"), Err(nom::Err::Error(nom::error::Error::new("__N51SW", ErrorKind::IsNot))));

     let config_limit_example = ConfiguredLimits {
         volts_hi_times_ten: 155,
         volts_lo_times_ten: 130,
         dif: 400,
         cht: 415,
         cld: 60,
         tit: 1650,
         oil_hi: 220,
         oil_lo: 75
     };
     assert_eq!(configured_limits_parser("155,130,400,415, 60,1650,220, 75"), Ok(("", config_limit_example)));
     assert_eq!(parse_record("$A,155,130,400,415, 60,1650,220, 75*70"), Ok(("", HeaderRecord::A(config_limit_example))));

     let fuel_flow_example = FuelFlowLimits {
         empty: 0,
         full: 49,
         warning: 22,
         k_factor: 3183,
         k_factor2: 3183,
     };
     assert_eq!(fuel_flow_parser("0, 49, 22,3183,3183"), Ok(("", fuel_flow_example)));
     assert_eq!(parse_record("$F,0, 49, 22,3183,3183*57"), Ok(("", HeaderRecord::F(fuel_flow_example))));

     let timestamp_example = Timestamp {
         month: 5,
         day: 13,
         year: 5,
         hour: 23,
         minute: 2,
         unknown: 2222,
     };
     assert_eq!(timestamp_parser("5,13, 5,23, 2, 2222"), Ok(("", timestamp_example)));
     assert_eq!(parse_record("$T, 5,13, 5,23, 2, 2222*65"), Ok(("", HeaderRecord::T(timestamp_example))));

     let config_info_example = ConfigInfo {
         model_number: 700,
         feature_flags_lo: 63741,
         feature_flags_hi: 6193,
         unknown_flags: 1552,
         firmware_version: 292,
     };
     assert_eq!(config_info_parser("700,63741, 6193, 1552, 292"), Ok(("", config_info_example)));
     assert_eq!(parse_record("$C, 700,63741, 6193, 1552, 292*58"), Ok(("", HeaderRecord::C(config_info_example))));

     let flight_info_example = FlightInfo {
         flight_number: 227,
         length: 3979,
     };
     assert_eq!(flight_info_parser("227, 3979"), Ok(("", flight_info_example)));
     assert_eq!(parse_record("$D,  227, 3979*57"), Ok(("", HeaderRecord::D(flight_info_example))));

     let last_header_record_example = LastHeaderRecord {
         unknown: 49,
     };
     assert_eq!(last_header_record_parser("49"), Ok(("", last_header_record_example)));
     assert_eq!(parse_record("$L, 49*4D"), Ok(("", HeaderRecord::L(last_header_record_example))));
 }

fn main() {
    let raw: &str = "$U,N51SW__*37";
    println!("{:?}", header_record_parser(raw));

    println!("{:?}", configured_limits_parser("155,130,400,415, 60,1650,220, 75"));
}
