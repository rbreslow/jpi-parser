use std::io::{BufReader, Read, ErrorKind};
use std::io;
use std::fs::{File, read};
use std::mem::size_of;
use std::error::Error;
use nom::error::ParseError;

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


pub fn read_flight_header(reader: &mut BufReader<File>) -> io::Result<flightheader> {
    let mut buf = [0u8; size_of::<flightheader>()];
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

    Ok(flightheader {
        flightnumber,
        flags,
        unknown,
        interval_secs,
        datebits,
        timebits
    })
}

fn read_data_header(reader: &mut BufReader<File>) -> io::Result<data_header> {
    let mut header_bytes = [0u8; size_of::<data_header>()];
    reader.read_exact(&mut header_bytes)?;

    Ok(data_header {
        decodeflags: [header_bytes[0], header_bytes[1]],
        repeatcount: header_bytes[2]
    })
}

pub fn read_next_data(prev: &[u16; 48], reader: &mut BufReader<File>) -> io::Result<[u16; 48]> {
    let header = read_data_header(reader)?;
    if header.repeatcount != 0 {
        return Ok(*prev);
    }
    const MAX_FLAG_BYTES: usize = 14; // 6 + 2 + 6
    let num_field_flags = (header.decodeflags[0] & 0x3F).count_ones() as usize; // never greater than 6
    let num_scale_flags = ((header.decodeflags[0] & 0xC0) >> 6).count_ones() as usize; // never greater than 2
    let mut flag_buffer = [0u8; MAX_FLAG_BYTES];
    reader.read_exact(&mut flag_buffer[0..(num_field_flags * 2 + num_scale_flags)])?;

    let mut offset = 0usize;
    let field_flags = &flag_buffer[offset..num_field_flags];
    offset += num_field_flags;
    let scale_flags = &flag_buffer[offset..offset+num_scale_flags];
    offset += num_scale_flags;
    let sign_flags = &flag_buffer[offset..offset+num_field_flags];

    let num_fields = field_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let mut field_dif_buffer = [0u8; 48]; // num_fields is how much of this buffer is actually used
    reader.read_exact( &mut field_dif_buffer[0usize..num_fields])?;

    let num_scale = scale_flags.iter().map(|x| x.count_ones()).sum::<u32>() as usize;
    let mut scale_dif_buffer = [0u8; 16];
    reader.read_exact(&mut scale_dif_buffer[0usize..num_scale])?;

    let mut out = *prev;
    let mut dif_buffer_idx = 0usize; // index to field_dif_buffer scale_dif_buffer
    for i in 0..num_field_flags { // apply field dif
        for bit in 0..8 { // this can be optimized with leading_zeros
            //let mut flag = field_flags[i];
            if ((field_flags[i] >> bit) & 1) != 0 {
                let mut diff = 0u16;
                if i < num_scale_flags {
                    if ((scale_flags[i] >> bit) & 1) != 0 {
                        diff = (scale_dif_buffer[dif_buffer_idx] as u16) << 8; // set high order byte
                    }
                }

                let sign: bool = ((sign_flags[i] >> bit) & 1) != 0;
                let idx = (i * 8) + bit;
                diff |= field_dif_buffer[dif_buffer_idx] as u16; // set low byte
                if sign {
                    out[idx] -= diff;
                } else {
                    out[idx] += diff;
                }

                dif_buffer_idx += 1;
            }
        }
    }
    let mut checksum = [0u8; 1];
    reader.read_exact(&mut checksum)?;
    // TODO: validate checksum (not explained in document but is probably just xor)

    Ok(out)
}

