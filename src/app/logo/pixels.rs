pub(crate) const SIDE: usize = 16;

pub(crate) const ROWS: [&str; SIDE] = [
    "...BBBBBBBBBB...",
    ".BBBBBBBBBBBBBB.",
    ".BBBBKKKKBBBBBB.",
    "BBBKKKKKKKBBBBBB",
    "BBKKKKKKKKKBBBBB",
    "BBKKKKKKWKKBBBBB",
    "BBKKKKKKKKKOOOKB",
    "BBKKKKWWWWWLLBBB",
    "BBKKWWWWWWBBBBBB",
    "BBGWWWWWWBBBBBBB",
    "BBGWWWWWBBBBBBBB",
    "BBGWWWWWBBBBBBBB",
    "BBGWWWWWBBBBBBBB",
    ".GWWWWWWWBBBBBB.",
    ".GWWWWWWWBBBBBB.",
    "...WWWWWWBBBB...",
];

const SKY: u32 = 0x8ec3e6;
const CAP: u32 = 0x111111;
const FEATHERS: u32 = 0xffffff;
const NAPE_SHADE: u32 = 0xc9d3dc;
const UPPER_BILL: u32 = 0xf26b1d;
const LOWER_BILL: u32 = 0xf7893a;

pub(crate) fn rgb(pixel: u8) -> Option<u32> {
    match pixel {
        b'B' => Some(SKY),
        b'K' => Some(CAP),
        b'W' => Some(FEATHERS),
        b'G' => Some(NAPE_SHADE),
        b'O' => Some(UPPER_BILL),
        b'L' => Some(LOWER_BILL),
        _ => None,
    }
}
