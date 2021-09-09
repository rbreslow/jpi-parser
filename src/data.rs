use std::io::{BufReader, Read};
use std::io;
use std::fs::{File, read};
use std::mem::size_of;
use std::error::Error;
use nom::error::ParseError;
use nom::IResult;
use nom::number::complete as num;
use nom::bytes::complete as bytes;

use crate::headers::{ConfigInfo, num_cyls, num_engines};
use std::ops::Range;
use std::cmp::{min, max};


#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(packed)]
pub struct flightheader {
    flightnumber: u16,
    flags: u32, // not actually in the file as a big endian 32 bit int
    unknown: u16,
    interval_secs: u16,
    datebits: u16,
    timebits: u16
}


#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct data_record {
    // first byte of flags
    pub egt: [i16; 6],
    pub t1: i16,
    pub t2: i16,

    // second byte of flags
    pub cht: [i16; 6],
    pub cld: i16,
    pub oil: i16,

    // third byte of flags
    pub mark: i16,
    pub unk_3_1: i16,
    pub cdt: i16,
    pub iat: i16,
    pub bat: i16,
    pub oat: i16,
    pub usd: i16,
    pub ff: i16,

    // fourth byte of flags
    pub regt: [i16; 6],
    pub hp_rt1: i16, // hp/rt1 union
    pub rt2: i16,

    // fifth byte of flags
    pub rcht: [i16; 6],
    pub rcld: i16,
    pub roil: i16,

    // sixth byte of flags
    pub map: i16,
    pub rpm: i16,
    pub rpm_highbyte_rcdt: i16, // rpm_highbyte/rcdt union
    pub riat: i16,
    pub unk_6_4: i16,
    pub unk_6_5: i16,
    pub rusd: i16,
    pub rff: i16
}

const TWINJUMP: u32 = 3 * 8; // offset from egt to regt

fn has_rpm(header: &flightheader) -> bool {
    const RPM_BIT: u32 = 1 << 26;
    header.flags & RPM_BIT == RPM_BIT
}

impl data_record {
    fn as_array(&mut self) -> &mut [i16; 48] {
        unsafe { std::mem::transmute(self) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct binary_record {
    pub data: data_record,
    pub dif: [i16; 2],
    pub naflags: [u8; 6]
}

impl binary_record {
    pub fn new(config: &ConfigInfo) -> binary_record {
        let mut data = data_record::default();
        data.as_array().fill(0xF0);
        if num_engines(config) == 1 {
            data.hp_rt1 = 0; // hp = 0
            data.rpm_highbyte_rcdt = 0; // rpm_highbyte = 0
        }

        binary_record {
            data,
            dif: [0i16; 2],
            naflags: [0u8; 6] // not available flags
        }
    }

    // im just pasting the reference impl lol
    pub fn calcstuff(&mut self, config: &ConfigInfo, header: &flightheader) {
        let cyls = num_cyls(header.flags);
        let engines = num_engines(config);
        assert!(cyls <= 6 || engines == 1);

        for j in 0..engines {
            let mut emax = -1i16; let mut emin = 0x7FFFi16;
            for i in 0..cyls {
                let idx = if i < 6 { i + j * TWINJUMP } else { i - 6 + TWINJUMP } as usize;
                if !test_bit(self.naflags[idx / 8], (idx % 8) as u32) {
                    emin = min(emin, self.data.egt[idx]);
                    emax = max(emax, self.data.egt[idx]);
                }
            }
            self.dif[j as usize] = emax - emin;
        }

        if has_rpm(header) {
            self.data.rpm += (self.data.rpm_highbyte_rcdt << 8);
            self.data.rpm_highbyte_rcdt = 0;
        }
    }
}

// every binary record begins with this and this tells how many flag bytes to read
struct data_header {
    // [1] should apparently always == [0]
    // bits 0-5 are for fieldflags/signflags
    // bits 6-7 are for scaleflags
    decodeflags: [u8; 2],
    repeatcount: u8,
}

fn be_u16_uwu(slice: &[u8]) -> u16 {
    ((slice[0] as u16) << 8) | slice[1] as u16
}

fn be_u32_uwu(slice: &[u8]) -> u32 {
    ((slice[0] as u32) << (8 * 3)) |
    ((slice[1] as u32) << (8 * 2)) |
    ((slice[2] as u32) << (8 * 1)) |
    ((slice[3] as u32))
}

fn calc_new_checksum(data: &[u8]) -> u8 {
    let sum: u8 = data.iter().fold(0u8, |acc, x| acc.overflowing_add(*x).0);
    (-(sum as i8)) as u8
}

fn calc_checksum(data: &[u8]) -> u8 {
    return calc_new_checksum(data);
}

pub fn read_flight_header(reader: &mut BufReader<File>) -> io::Result<flightheader> {
    let mut buf = [0u8; size_of::<flightheader>() + 1];
    reader.read_exact(&mut buf)?;

    let mut i = 0usize;
    let flightnumber = be_u16_uwu(&buf[i..]);
    i += 2;
    let flags_lo = be_u16_uwu(&buf[i..]);
    i += 2;
    let flags_hi = be_u16_uwu(&buf[i..]);
    i += 2;
    let unknown = be_u16_uwu(&buf[i..]);
    i += 2;
    let interval_secs = be_u16_uwu(&buf[i..]);
    i += 2;
    let datebits = be_u16_uwu(&buf[i..]);
    i += 2;
    let timebits = be_u16_uwu(&buf[i..]);
    i += 2;
    let checksum = buf[i];
    let computed = calc_checksum(&buf[..size_of::<flightheader>()]);
    assert_eq!(checksum, computed);

    Ok(flightheader {
        flightnumber,
        flags: (flags_hi as u32) << 16 | (flags_lo as u32),
        unknown,
        interval_secs,
        datebits,
        timebits
    })
}

fn parse_data_header(i: &[u8]) -> IResult<&[u8], data_header> {
    let (i, decode1) = num::u8(i)?;
    let (i, decode2) = num::u8(i)?;
    let (i, repeat) = num::u8(i)?;
    if decode1 != decode2 {
        panic!("mismatched decode bytes") // TODO: remove this
    }

    Ok((i, data_header {
        decodeflags: [decode1, decode2],
        repeatcount: repeat
    }))
}

fn test_bit(x: u8, bit: u32) -> bool {
    ((x >> bit) & 1) != 0
}

fn test_bit_slice(arr: &[u8], bit: u32) -> bool {
    test_bit(arr[(bit / 8) as usize], bit % 8)
}

fn clear_bit(x: &mut u8, bit: u32) {
    *x &= !(1 << bit);
}

fn clear_bit_slice(arr: &mut [u8], bit: u32) {
    clear_bit(&mut arr[(bit / 8) as usize], bit % 8)
}

fn set_bit(x: &mut u8, bit: u32) {
    *x |= 1 << bit;
}

fn set_bit_slice(arr: &mut [u8], bit: u32) {
    set_bit(&mut arr[(bit / 8) as usize], bit % 8);
}

fn parse_decode_bits<'a>(i: &'a[u8], out: &mut [u8], decodeflags: u8, bits: Range<u8>) -> IResult<&'a [u8], ()> {
    let mut i = i;
    for bit in bits.clone() {
        if test_bit(decodeflags, bit as u32) {
            let (j, flags) = num::u8(i)?;
            i = j;
            let idx = bit - bits.start;
            out[idx as usize] = flags;
        }
    }
    Ok((i, ()))
}

pub fn parse_binary_record<'a>(prev: &binary_record, input: &'a [u8], config: &ConfigInfo, fheader: &flightheader) -> IResult<&'a [u8], binary_record> {
    assert_eq!(((config.feature_flags_hi as u32) << 16 | (config.feature_flags_lo as u32)), fheader.flags);

    let (i, header) = parse_data_header(input)?;
    if header.repeatcount != 0 {
        if header.repeatcount > 1 { // TODO: this isn't handled properly
            unimplemented!()
        }
        return Ok((i, *prev));
    }
    let mut field_flags = [0u8; 6];
    let mut scale_flags = [0u8; 2];
    let mut sign_flags = [0u8; 6];

    let (i, _) = parse_decode_bits(i, &mut field_flags, header.decodeflags[0], 0..6)?;
    let (i, _) = parse_decode_bits(i, &mut scale_flags, header.decodeflags[0], 6..8)?;
    let (i, _) = parse_decode_bits(i, &mut sign_flags,  header.decodeflags[0], 0..6)?;
    assert!(scale_flags[1] == 0 || num_engines(config) == 1);

    let num_fields = field_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let (i, field_dif) = bytes::take(num_fields)(i)?;

    let num_scale = scale_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let (i, scale_dif) = bytes::take(num_scale)(i)?;

    let mut out = *prev;

    let mut field_dif_idx = 0usize; // index to field_dif and scale_dif
    for i in 0..field_flags.len() { // apply field dif
        let mut flag = field_flags[i];
        while flag != 0 {
            let bit = flag.trailing_zeros();

            let sign = test_bit(sign_flags[i], bit);
            let idx = (i * 8) + bit as usize;
            let diff = field_dif[field_dif_idx] as i16; // set low byte
            if diff != 0 {
                set_bit(&mut out.naflags[i], bit);
            } else {
                clear_bit(&mut out.naflags[i], bit);
            }

            let array = out.data.as_array();

            if sign {
                array[idx] = array[idx].overflowing_sub(diff).0; // -
            } else {
                array[idx] = array[idx].overflowing_add(diff).0; // +
            }

            field_dif_idx += 1;
            clear_bit(&mut flag, bit);
        }
    }

    let mut scale_dif_idx = 0usize;
    for f in 0..scale_flags.len() {
        for bit in 0..8 {
            if test_bit(scale_flags[f], bit) {
                let idx = f as u32 * TWINJUMP + bit;
                let mut x = scale_dif[scale_dif_idx] as i16;
                if x != 0 {
                    clear_bit_slice(&mut out.naflags, idx);
                    x <<= 8;
                    if test_bit_slice(&sign_flags, idx) {
                        out.data.as_array()[idx as usize] -= x;
                    } else {
                        out.data.as_array()[idx as usize] += x;
                    }
                }

                scale_dif_idx += 1;
            }
        }
    }

    if num_engines(config) == 1 {
        if test_bit(sign_flags[5], 1) { // rpm
            assert!(!test_bit(sign_flags[5], 2)); // rpm_highbyte
            out.data.rpm_highbyte_rcdt = -out.data.rpm_highbyte_rcdt;
            if out.data.rpm_highbyte_rcdt != 0 {
                clear_bit(&mut out.naflags[5], 1); // rpm
            }
        }
    }
    out.calcstuff(config, fheader);

    let end_ptr = i.as_ptr(); // dont want to include the checksum
    let (i, checksum) = num::u8(i)?;
    let begin_ptr = input.as_ptr();
    let record_size = unsafe { end_ptr.offset_from(begin_ptr) } as usize;
    let all_bytes = unsafe { std::slice::from_raw_parts(begin_ptr, record_size) };
    let calculated = calc_checksum(all_bytes);
    assert_eq!(checksum, calculated);

    Ok((i, out))
}

