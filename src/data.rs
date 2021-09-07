use std::io::{BufReader, Read, ErrorKind};
use std::io;
use std::fs::{File, read};
use std::mem::size_of;
use std::error::Error;
use nom::error::ParseError;
use nom::IResult;
use nom::number::complete as num;
use nom::bytes::complete as bytes;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(packed)]
pub struct flightheader {
    flightnumber: u16,
    flags: u32,
    unknown: u16,
    interval_secs: u16,
    datebits: u16,
    timebits: u16
}

// TODO: work with more than 6 cylinders
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct data_record {
    // first byte of flags
    pub egt: [u16; 6],
    pub t1: u16,
    pub t2: u16,

    // second byte of flags
    pub cht: [u16; 6],
    pub cld: u16,
    pub oil: u16,

    // third byte of flags
    pub mark: u16,
    pub unk_3_1: u16,
    pub cdt: u16,
    pub iat: u16,
    pub bat: u16,
    pub oat: u16,
    pub usd: u16,
    pub ff: u16,

    // fourth byte of flags
    pub regt: [u16; 6],
    pub hp_rt1: u16, // hp/rt1 union
    pub rt2: u16,

    // fifth byte of flags
    pub rcht: [u16; 6],
    pub rcld: u16,
    pub roil: u16,

    // sixth byte of flags
    pub map: u16,
    pub rpm: u16,
    pub rpm_highbyte_rcdt: u16, // rpm_highbyte/rcdt union
    pub riat: u16,
    pub unk_6_4: u16,
    pub unk_6_5: u16,
    pub rusd: u16,
    pub rff: u16
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
    let flags = be_u32_uwu(&buf[i..]);
    i += 4;
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
        flags,
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

pub fn parse_binary_record<'a>(prev: &[u16; 48], input: &'a [u8]) -> IResult<&'a [u8], [u16; 48]> {

    let (i, header) = parse_data_header(input)?;
    if header.repeatcount != 0 {
        return Ok((i, *prev));
    }
    let num_field_flags = (header.decodeflags[0] & 0x3F).count_ones() as usize; // never greater than 6
    let num_scale_flags = ((header.decodeflags[0] & 0xC0) >> 6).count_ones() as usize; // never greater than 2

    let (i, field_flags) = bytes::take(num_field_flags)(i)?;
    let (i, scale_flags) = bytes::take(num_scale_flags)(i)?;
    let (i, sign_flags) = bytes::take(num_field_flags)(i)?;

    let num_fields = field_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let (i, field_dif) = bytes::take(num_fields)(i)?;

    let num_scale = scale_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let (i, scale_dif) = bytes::take(num_scale)(i)?;

    let mut out = *prev;
    let mut dif_slice_idx = 0usize; // index to field_dif_buffer and scale_dif_buffer
    for i in 0..num_field_flags { // apply field dif
        let mut flag = field_flags[i];
        while flag != 0 {
            let bit = flag.trailing_zeros();
            let mut diff = 0u16;
            if i < num_scale_flags {
                if ((scale_flags[i] >> bit) & 1) != 0 {
                    diff = (scale_dif[dif_slice_idx] as u16) << 8; // set high order byte
                }
            }

            let sign: bool = ((sign_flags[i] >> bit) & 1) != 0;
            let idx = (i * 8) + bit as usize;
            diff |= field_dif[dif_slice_idx] as u16; // set low byte
            if sign {
                out[idx] = out[idx].overflowing_sub(diff).0; // -
            } else {
                out[idx] = out[idx].overflowing_add(diff).0; // +
            }

            dif_slice_idx += 1;
            flag &= !(1 << bit); // zero the bit
        }
    }
    let end_ptr = i.as_ptr(); // dont want to include the checksum
    let (i, checksum) = num::u8(i)?;
    let begin_ptr = input.as_ptr();
    let record_size = unsafe { end_ptr.offset_from(begin_ptr) } as usize;
    let all_bytes = unsafe { std::slice::from_raw_parts(begin_ptr, record_size) };
    let calculated = calc_checksum(all_bytes);
    assert_eq!(checksum, calculated);

    Ok((i, out))
}

