use std::io::{BufReader, Read, ErrorKind};
use std::io;
use std::fs::{File, read};
use std::mem::size_of;
use std::error::Error;

pub struct flightheader {
    flightnumber: u16,
    flags: u64,
    unknown: u16,
    interval_secs: u16,
    datebits: u16,
    timebits: u16
}

// TODO: work with more than 6 cylinders
struct data_record {
    // first byte of flags
    egt: [u16; 6],
    t1: u16,
    t2: u16,

    // second byte of flags
    cht: [u16; 6],
    cld: u16,
    oil: u16,

    // third byte of flags
    mark: u16,
    unk_3_1: u16,
    cdt: u16,
    iat: u16,
    bat: u16,
    oat: u16,
    usd: u16,
    ff: u16,

    // fourth byte of flags
    regt: [u16; 6],
    hp_rt1: u16, // hp/rt1 union
    rt2: u16,

    // fifth byte of flags
    rcht: [u16; 6],
    rcld: u16,
    roil: u16,

    // sixth byte of flags
    map: u16,
    rpm: u16,
    rpm_highbyte_rcdt: u16, // rpm_highbyte/rcdt union
    riat: u16,
    unk_6_4: u16,
    unk_6_5: u16,
    rusd: u16,
    rff: u16
}

// every binary record begins with this and this tells how many flag bytes to read
struct data_header {
    // [1] should apparently always == [0]
    // bits 0-5 are for fieldflags/signflags
    // bits 6-7 are for scaleflags
    decodeflags: [u8; 2],
    repeatcount: u8,
}

fn read_data_header(reader: &mut BufReader<File>) -> io::Result<data_header> {
    const SIZE: usize = size_of::<data_header>();
    let mut header_bytes = [0u8; SIZE];
    reader.read_exact(&mut header_bytes)?;

    Ok(data_header {
        decodeflags: [header_bytes[0], header_bytes[1]],
        repeatcount: header_bytes[2]
    })
}

fn read_next_data(reader: &mut BufReader<File>) -> io::Result<data_record> {
    let header = read_data_header(reader)?;
    if header.repeatcount != 0 {
        unimplemented!()
    }
    const MAX_FLAG_BYTES: usize = 14;
    let num_field_flags = (header.decodeflags[0] & 0x3F).count_ones() as usize;
    let num_scale_flags = ((header.decodeflags[0] & 0xC0) >> 6).count_ones() as usize;
    let mut flag_buffer = [0u8; MAX_FLAG_BYTES];
    reader.read_exact(&mut flag_buffer[0..(num_field_flags * 2 + num_scale_flags)])?;

    let mut offset = 0usize;
    let field_flags = &flag_buffer[offset..num_field_flags];
    offset += num_field_flags;
    let scale_flags = &flag_buffer[offset..offset+num_scale_flags];
    offset += num_scale_flags;
    let sign_flags = &flag_buffer[offset..num_field_flags];


}

