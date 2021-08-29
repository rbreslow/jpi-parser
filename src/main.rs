use nom::{Err, IResult, Parser};
use nom::bytes::complete::{is_not, tag, take, take_until};
use nom::error::{Error, ErrorKind};

#[derive(Clone, Default, Debug)]
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

 #[derive(Clone, Debug)]
 enum HeaderRecordType<'a> {
     U(&'a str),
     A(ConfiguredLimits)
 }

 #[derive(Clone, Default, Debug)]
 struct FuelFlowLimits {
     empty: u16,
     full: u16,
     warning: u16,
     k_factor: u16,
     k_factor2: u16,
 }

 #[derive(Clone, Default, Debug)]
 struct Timestamp {
     month: u16,
     day: u16,
     year: u16,
     hour: u16,
     minute: u16,
     unknown: u16,
 }

 #[derive(Clone, Default, Debug)]
 struct ConfigInfo {
     model_number: u16,
     feature_flags_lo: u16,
     feature_flags_hi: u16,
     unknown_flags: u16,
     firmware_version: u16,
 }

 #[derive(Clone, Default, Debug)]
 struct FlightInfo {
     flight_number: u16,
     length: u16
 }

 #[derive(Clone, Default, Debug)]
 struct LastHeaderRecord {
     unknown: u16
 }

 fn not_underscore(i: &str) -> nom::IResult<&str, &str> {
     is_not("_")(i)
 }

 fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
     u8::from_str_radix(input, 16)
 }

 fn tail_number_parser(i: &str) -> IResult<&str, (&str, &str)> {
     let (i, header_record_type) = take(1usize)(i)?;
     let (i, _) = tag(",")(i)?;
     let (i, tail_number) = not_underscore(i)?;

     Ok((i, (header_record_type, tail_number)))
 }

 fn configured_limits_parser(i: &str) -> IResult<&str, ConfiguredLimits> {
     Ok((i, ConfiguredLimits {
         volts_hi_times_ten: 0,
         volts_lo_times_ten: 0,
         dif: 0,
         cht: 0,
         cld: 0,
         tit: 0,
         oil_hi: 0,
         oil_lo: 0
     }))
 }

 fn header_record_parser(i: &str) -> IResult<&str, (&str, &str)> {
     let (i, _) = tag("$")(i)?;
     let (i, header_record_type) = take(1usize)(i)?;
     let (i, _) = take_until(",")(i)?;
     let (i, header_record) = take_until("*")(i)?;
     let (i, _) = tag("*")(i)?;
     let (i, checksum) = take(2usize)(i)?;

     let mut computed_checksum: u8 = 0;
     for byte in header_record.bytes() {
         computed_checksum ^= byte;
     }

     if computed_checksum == from_hex(checksum).unwrap() {
         println!("checksum is heckin valid")
     }

     Ok((i, (header_record_type, header_record)))
 }

 #[test]
 fn test() {
     assert_eq!(not_underscore("N51SW__"), Ok(("__", "N51SW")));
     assert_eq!(not_underscore("__N51SW"), Err(nom::Err::Error(nom::error::Error::new("__N51SW", ErrorKind::IsNot))));

     assert_eq!(tail_number_parser("U,N51SW__"), Ok(("__", ("U", "N51SW"))));
 }

fn main() {
    let raw: &str = "$U,N51SW__*37";
    println!("{:?}", header_record_parser(raw));
}
