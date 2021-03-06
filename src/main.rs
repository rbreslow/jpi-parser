mod headers;
mod data;

use headers::*;
use data::*;
use nom::error::ErrorKind;
use std::fs::File;
use std::io::{BufReader, Read, BufRead, Seek};
use std::mem::size_of;
use headers::HeaderRecord::*;


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
     assert_eq!(parse_header_record("$A,155,130,400,415, 60,1650,220, 75*70"), Ok(("", HeaderRecord::A(config_limit_example))));

     let fuel_flow_example = FuelFlowLimits {
         empty: 0,
         full: 49,
         warning: 22,
         k_factor: 3183,
         k_factor2: 3183,
     };
     assert_eq!(fuel_flow_parser("0, 49, 22,3183,3183"), Ok(("", fuel_flow_example)));
     assert_eq!(parse_header_record("$F,0, 49, 22,3183,3183*57"), Ok(("", HeaderRecord::F(fuel_flow_example))));

     let timestamp_example = Timestamp {
         month: 5,
         day: 13,
         year: 5,
         hour: 23,
         minute: 2,
         unknown: 2222,
     };
     assert_eq!(timestamp_parser("5,13, 5,23, 2, 2222"), Ok(("", timestamp_example)));
     assert_eq!(parse_header_record("$T, 5,13, 5,23, 2, 2222*65"), Ok(("", HeaderRecord::T(timestamp_example))));

     let config_info_example = ConfigInfo {
         model_number: 700,
         feature_flags_lo: 63741,
         feature_flags_hi: 6193,
         unknown_flags: 1552,
         firmware_version: 292,
     };
     assert_eq!(config_info_parser("700,63741, 6193, 1552, 292"), Ok(("", config_info_example)));
     assert_eq!(parse_header_record("$C, 700,63741, 6193, 1552, 292*58"), Ok(("", HeaderRecord::C(config_info_example))));

     let flight_info_example = FlightInfo {
         flight_number: 227,
         length: 3979,
     };
     assert_eq!(flight_info_parser("227, 3979"), Ok(("", flight_info_example)));
     assert_eq!(parse_header_record("$D,  227, 3979*57"), Ok(("", HeaderRecord::D(flight_info_example))));

     let last_header_record_example = LastHeaderRecord {
         unknown: 49,
     };
     assert_eq!(last_header_record_parser("49"), Ok(("", last_header_record_example)));
     assert_eq!(parse_header_record("$L, 49*4D"), Ok(("", HeaderRecord::L(last_header_record_example))));
 }

fn main() {
    let raw: &str = "$U,N51SW__*37";
    //println!("{:?}", header_record_parser(raw));

    //println!("{:?}", configured_limits_parser("155,130,400,415, 60,1650,220, 75"));

    let f = File::open("U210818.JPI").unwrap();
    let mut reader = BufReader::new(f);

    let mut header_records = Vec::new();

    for line in reader.by_ref().lines() {
        let (_, record) = parse_header_record(line.unwrap().as_str()).unwrap();
        let mut last = false;
        if let L(_) = record {
            last = true;
        }
        header_records.push(record);
        if last {
            break;
        }
    }

    let mut first_flight_len = 0usize;
    for record in &header_records {
        if let D(info) = record {
            first_flight_len = (info.length * 2) as usize; // convert number of shorts to number of bytes
            break;
        }
    }
    let config = header_records.iter().find_map(|h| match h {
        C(cfg) => Some(cfg),
        _ => None
    }).unwrap();

    println!("position {}", reader.stream_position().unwrap());
    let header = read_flight_header(&mut reader).unwrap();
    println!("{:?}", &header);
    println!("position {}", reader.stream_position().unwrap());
    println!("sizeof flightheader {}", size_of::<flightheader>());

    let init = binary_record::new(&config);

    let mut flight_data = vec![0u8; first_flight_len];
    reader.read(flight_data.as_mut_slice()).unwrap();
    let i = flight_data.as_slice();
    let (i, data1) = parse_binary_record(&init, i, &config, &header).unwrap();
    let (i, data2) = parse_binary_record(&data1, i, &config, &header).unwrap();
    println!("{:?}", &data1.data);
    println!("{:?}", &data2.data);
}
